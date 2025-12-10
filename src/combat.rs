use bevy::input::mouse::MouseButton;
use bevy::prelude::*;
use bevy::ui::Overflow;
use std::collections::HashMap;
use bevy::ecs::query::WorldQuery; // (如果尚未 import，可以忽略)

use crate::equipment::{EquipmentSet, WeaponKind};
use crate::health::Health;
use crate::input::MovementInput;
use crate::movement::Player;
use crate::state::GameState;
use crate::enemy::Enemy;   // 重點: 從 enemy 模組引入 Enemy

/// 战斗插件：普通攻击 + 公共 Slash 技能 + 弹幕（敌人定义在 enemy.rs 里）
pub struct CombatPlugin;

/// 攻击状态：管理普攻与技能冷却
#[derive(Component, Default)]
pub struct AttackState {
    /// 普通攻击冷却（秒）
    pub basic_cooldown: f32,
    /// Slash 技能冷却（目前留给技能系统按需使用）
    pub slash_cooldown: f32,
}

/// 弹幕组件
#[derive(Component)]
pub struct Projectile {
    pub direction: Vec2,
    pub speed: f32,
    pub lifetime: f32,
    pub damage: f32,
    pub from_player: bool,
}

/// Slash 技能特效组件
#[derive(Component)]
pub struct SlashVfx {
    pub timer: Timer,
}

#[derive(Component)]
pub struct EnemyHpBar {
    pub owner: Entity,
    /// 当前血量比例，0.0 - 1.0，用来表示填充宽度（百分比计算）
    pub ratio: f32,
}

/// 血条填充节点标记
#[derive(Component)]
pub struct EnemyHpBarFill;

/// 资源：敌人 -> 血条 实体 映射
#[derive(Resource, Default)]
pub struct EnemyHpBarMap(pub HashMap<Entity, Entity>);

impl Plugin for CombatPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<EnemyHpBarMap>();
        app.add_systems(
            Update,
            (
                ensure_attack_state,
                tick_attack_state,
                handle_basic_attack,
                cleanup_dead_enemies,
                update_projectiles,
                update_slash_vfx,
                sync_enemy_hp_bars,
                process_enemy_death,
            )
                .run_if(in_state(GameState::InGame)),
        );
    }
}

/// 递归删除实体及其子节点（用于删除 UI 根节点等）
/// 兼容 Bevy 0.17：Children::iter() 返回的是 Entity（按值）。
fn despawn_with_children(commands: &mut Commands, children_q: &Query<&Children>, entity: Entity) {
    // 如果 entity 有子节点，先递归删除它们
    if let Ok(children) = children_q.get(entity) {
        for child in children.iter() {
            // children.iter() 在 bevy 0.17 返回 Entity（按值），所以直接传 child
            despawn_with_children(commands, children_q, child);
        }
    }
    // 最后删除自己
    commands.entity(entity).despawn();
}

/// 确保玩家有 AttackState
fn ensure_attack_state(
    mut commands: Commands,
    query: Query<(Entity, Option<&AttackState>), With<Player>>,
) {
    for (entity, state) in &query {
        if state.is_none() {
            commands.entity(entity).insert(AttackState::default());
        }
    }
}

/// 冷却计时（普攻 + Slash）
fn tick_attack_state(time: Res<Time>, mut query: Query<&mut AttackState>) {
    let dt = time.delta_secs();
    for mut state in &mut query {
        if state.basic_cooldown > 0.0 {
            state.basic_cooldown = (state.basic_cooldown - dt).max(0.0);
        }
        if state.slash_cooldown > 0.0 {
            state.slash_cooldown = (state.slash_cooldown - dt).max(0.0);
        }
    }
}

/// 左键普通攻击：
/// - 近战：前方短矩形范围；
/// — 远程：发射子弹。
fn handle_basic_attack(
    mouse: Res<ButtonInput<MouseButton>>,
    movement: Res<MovementInput>,
    mut commands: Commands,
    mut player_q: Query<(&Transform, &EquipmentSet, &mut AttackState), With<Player>>,
    mut enemies_q: Query<(Entity, &Transform, &mut Health), With<Enemy>>,
) {
    if !mouse.just_pressed(MouseButton::Left) {
        return;
    }

    let Ok((player_tf, equip, mut state)) = player_q.single_mut() else {
        return;
    };

    if state.basic_cooldown > 0.0 {
        return;
    }

    let dir = if movement.0 != Vec2::ZERO {
        movement.0.normalize()
    } else {
        Vec2::Y
    };

    match equip.weapon_kind {
        WeaponKind::Melee => {
            let damage = equip.weapon_damage * 1.5;
            perform_melee_attack(
                player_tf.translation.truncate(),
                dir,
                equip.melee_range,
                equip.melee_width,
                damage,
                &mut enemies_q,
            );
        }
        WeaponKind::Ranged => {
            let damage = equip.weapon_damage * 1.3;
            spawn_projectile(
                &mut commands,
                player_tf.translation.truncate(),
                dir,
                equip.weapon_projectile_speed,
                equip.weapon_projectile_lifetime,
                damage,
            );
        }
    }

    state.basic_cooldown = equip.weapon_attack_cooldown;
}

/// 通用：近战矩形攻击
fn perform_melee_attack(
    origin: Vec2,
    dir: Vec2,
    length: f32,
    width: f32,
    damage: f32,
    enemies_q: &mut Query<(Entity, &Transform, &mut Health), With<Enemy>>,
) {
    let forward = dir.normalize_or_zero();
    if forward == Vec2::ZERO {
        return;
    }
    let right = Vec2::new(-forward.y, forward.x);

    for (_entity, tf, mut hp) in enemies_q.iter_mut() {
        let to_target = tf.translation.truncate() - origin;
        let d_forward = to_target.dot(forward);
        let d_side = to_target.dot(right);

        if d_forward >= 0.0 && d_forward <= length && d_side.abs() <= width * 0.5 {
            hp.current -= damage;
        }
    }
}

/// 公共技能：Slash（沿角色朝向的长条矩形攻击）
pub fn skill_slash(
    origin: Vec2,
    dir: Vec2,
    enemies_q: &mut Query<(Entity, &Transform, &mut Health), With<Enemy>>,
) {
    // 参数：你可以按需微调 length / width / damage
    let length: f32 = 260.0;
    let width: f32 = 100.0; // 把宽度略微放大一些，更不容易漏判
    let damage: f32 = 60.0;

    // 容忍值，用来补偿坐标/像素偏差
    const EPS: f32 = 6.0;

    // 容错：如果 dir 是零向量，使用一个默认朝向（朝上）
    let forward = {
        let f = dir.normalize_or_zero();
        if f == Vec2::ZERO {
            Vec2::Y
        } else {
            f
        }
    };

    // 垂直方向（右手方向）
    let right = Vec2::new(-forward.y, forward.x);

    // 为了让判定更直观（和视觉对齐），把判定区间当作
    // 从 origin 开始到 origin + forward * length（允许小的负向 EPS）
    for (entity, tf, mut hp) in enemies_q.iter_mut() {
        let to_target = tf.translation.truncate() - origin;
        let d_forward = to_target.dot(forward);
        let d_side = to_target.dot(right);

        // 放宽一下前向范围：允许略微的负偏差 EPS（使靠近玩家的目标也能被扫到），
        // 同时右侧判断使用 width / 2（单位同世界单位）
        if d_forward >= -EPS && d_forward <= length + EPS && d_side.abs() <= (width * 0.5 + EPS)
        {
            // apply damage
            hp.current -= damage;

            // 日志：方便调试（运行时查看控制台）
            info!(
                "skill_slash hit entity {}: -{:.1} hp -> {:.1}",
                entity.index(),
                damage,
                hp.current
            );
        } else {
            // 可选：在 debug 情况下输出每个实体的判定信息（注释掉避免大量日志）
            // info!(
            //     "skill_slash miss entity {}: d_forward={:.1}, d_side={:.1}",
            //     entity.index(),
            //     d_forward,
            //     d_side
            // );
        }
    }
}


/// Slash 的简易特效
pub fn spawn_slash_vfx(commands: &mut Commands, origin: Vec2, dir: Vec2) {
    let forward = dir.normalize_or_zero();
    if forward == Vec2::ZERO {
        return;
    }

    let length: f32 = 260.0;
    let width: f32 = 80.0;

    let mut sprite = Sprite::default();
    sprite.color = Color::srgba(0.9, 0.9, 0.3, 0.8);
    sprite.custom_size = Some(Vec2::new(length, width));

    // 计算矩形中心：在角色前方 length / 2 处
    let center = origin + forward * (length * 0.5);
    let angle = forward.y.atan2(forward.x);

    commands.spawn((
        sprite,
        Transform {
            translation: center.extend(15.0),
            rotation: Quat::from_rotation_z(angle),
            ..Default::default()
        },
        SlashVfx {
            timer: Timer::from_seconds(0.2, TimerMode::Once),
        },
    ));
}

/// 更新 Slash 特效
fn update_slash_vfx(
    time: Res<Time>,
    mut commands: Commands,
    mut q: Query<(Entity, &mut SlashVfx)>,
) {
    let dt = time.delta();
    for (entity, mut vfx) in &mut q {
        vfx.timer.tick(dt);
        if vfx.timer.is_finished() {
            commands.entity(entity).despawn();
        }
    }
}

/// 生成子弹
fn spawn_projectile(
    commands: &mut Commands,
    origin: Vec2,
    dir: Vec2,
    speed: f32,
    lifetime: f32,
    damage: f32,
) {
    let forward = dir.normalize_or_zero();
    if forward == Vec2::ZERO {
        return;
    }

    let mut sprite = Sprite::default();
    sprite.color = Color::srgb(1.0, 0.2, 0.2);
    sprite.custom_size = Some(Vec2::splat(8.0));

    commands.spawn((
        Projectile {
            direction: forward,
            speed,
            lifetime,
            damage,
            from_player: true,
        },
        sprite,
        Transform::from_xyz(origin.x, origin.y, 10.0),
    ));
}

/// 清理已经死亡的敌人（Health.current <= 0）
fn cleanup_dead_enemies(mut commands: Commands, enemies: Query<(Entity, &Health), With<Enemy>>) {
    for (entity, hp) in &enemies {
        if hp.current <= 0.0 {
            info!("Enemy {} died (hp {:.1}), despawning", entity.index(), hp.current);
            // 简单 despawn：会移除该实体及其组件
            // 如果你需要递归删除它的 UI 子节点或其它 child 实体，
            // 可以在这里调用专门的递归函数（例如 despawn_recursive）或
            // 使用 `commands.entity(entity).despawn_recursive()`（若可用）
            commands.entity(entity).despawn();
        }
    }
}

/// 更新子弹：移动 + 命中 + 销毁
fn update_projectiles(
    time: Res<Time>,
    mut commands: Commands,
    mut proj_q: Query<(Entity, &mut Projectile, &mut Transform), Without<Enemy>>,
    mut enemies_q: Query<
        (Entity, &Transform, &mut Health),
        (With<Enemy>, Without<Projectile>),
    >,
) {
    let dt = time.delta_secs();

    for (proj_entity, mut proj, mut tf) in &mut proj_q {
        proj.lifetime -= dt;
        if proj.lifetime <= 0.0 {
            commands.entity(proj_entity).despawn();
            continue;
        }

        let delta = proj.direction * proj.speed * dt;
        tf.translation.x += delta.x;
        tf.translation.y += delta.y;

        let hit_radius = 12.0;

        if proj.from_player {
            let mut hit_something = false;
            for (_enemy_entity, enemy_tf, mut hp) in &mut enemies_q {
                let dist =
                    enemy_tf.translation.truncate().distance(tf.translation.truncate());
                if dist <= hit_radius {
                    hp.current -= proj.damage;
                    hit_something = true;
                }
            }

            if hit_something {
                commands.entity(proj_entity).despawn();
            }
        }
    }
}

fn sync_enemy_hp_bars(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    enemies_q: Query<(Entity, &Health), With<Enemy>>,
    mut bar_map: ResMut<EnemyHpBarMap>,
    // 使用 ParamSet 把两个可变 Node 查询放在一起
    mut param_set: ParamSet<(
        Query<(&mut Node, &Children), With<EnemyHpBar>>, // p0: 根节点（可变 Node）
        Query<&mut Node, With<EnemyHpBarFill>>,         // p1: 填充节点（可变 Node）
    )>,
    children_q: Query<&Children>, // 用于递归删除
) {
    // 1) 收集本帧需要显示血条的“受伤但未死”的敌人
    let mut damaged: Vec<(Entity, f32, f32)> = Vec::new();
    for (e, hp) in &enemies_q {
        if hp.current > 0.0 && hp.current < hp.max {
            damaged.push((e, hp.current, hp.max));
        }
    }
    damaged.sort_by_key(|(e, _, _)| e.index());

    // 用于标记这一帧仍需保留的 owner
    let mut keep_set: std::collections::HashSet<Entity> = std::collections::HashSet::new();

    // 2) 为每个受伤敌人找到或创建血条，并更新位置/填充
    for (i, (enemy_entity, current, max)) in damaged.iter().enumerate() {
        keep_set.insert(*enemy_entity);

        // 找到已有血条或新建
        let bar_e = if let Some(&bar_e) = bar_map.0.get(enemy_entity) {
            bar_e
        } else {
            // 创建新的血条根节点（带子节点：填充条 + 文本）
            let font = asset_server.load("fonts/YuFanLixing.otf");
            let bar_e = commands
                .spawn((
                    Node {
                        position_type: PositionType::Absolute,
                        top: Val::Px(10.0 + i as f32 * 22.0),
                        left: Val::Px(20.0),
                        width: Val::Px(200.0),
                        height: Val::Px(16.0),
                        ..Default::default()
                    },
                    BackgroundColor(Color::srgba(0.12, 0.12, 0.12, 0.95)),
                    EnemyHpBar { owner: *enemy_entity, ratio: 1.0 },
                ))
                .with_children(|parent| {
                    // 填充条（子节点）
                    parent.spawn((
                        Node {
                            width: Val::Percent(100.0),
                            height: Val::Percent(100.0),
                            ..Default::default()
                        },
                        BackgroundColor(Color::srgba(0.8, 0.0, 0.0, 0.95)),
                        EnemyHpBarFill,
                    ));

                    // 文本（可选）
                    parent.spawn((
                        Text::new("Enemy".to_string()),
                        TextFont {
                            font: font.clone(),
                            font_size: 10.0,
                            ..default()
                        },
                        TextColor(Color::WHITE),
                    ));
                })
                .id();

            bar_map.0.insert(*enemy_entity, bar_e);
            bar_e
        };

        if let Ok((mut bar_node, children)) = param_set.p0().get_mut(bar_e) {
            // 把对 p0 的可变借用限制在一个小作用域，收集子节点
            let child_entities: Vec<Entity>;
            {
             // 更新根节点位置
            bar_node.top = Val::Px(10.0 + i as f32 * 22.0);
            bar_node.left = Val::Px(20.0);

            // 计算填充比例
            let ratio = ((*current / *max).clamp(0.0, 1.0)) * 100.0;

            // 复制 children 到一个独立的 Vec<Entity>（children.iter() 在 0.17 返回 Entity）
            child_entities = children.iter().collect::<Vec<Entity>>();

            // 注意：不要在这里调用 param_set.p1()，否则仍然会发生第二个可变借用冲突
            // 这里只收集数据并更新 bar_node，随后离开作用域释放 p0 的借用
        } // bar_node, children 的借用在这里结束

        // 现在可以安全地用 p1 去修改填充节点（因为 p0 的借用已释放）
        let ratio = ((*current / *max).clamp(0.0, 1.0)) * 100.0;
            for child in child_entities {
                if let Ok(mut fill_node) = param_set.p1().get_mut(child) {
                fill_node.width = Val::Percent(ratio);
            }
        }
    }
}

    // 3) 清理不再需要的血条（那些血条的 owner 不在 keep_set 中）
    // 先把现有映射收集，避免在迭代时修改哈希表
    let existing: Vec<(Entity, Entity)> = bar_map.0.iter().map(|(k, v)| (*k, *v)).collect();
    for (owner, bar_e) in existing {
        if !keep_set.contains(&owner) {
            // 移除映射
            bar_map.0.remove(&owner);
            // 递归删除血条根节点（以及其子节点）
            despawn_with_children(&mut commands, &children_q, bar_e);
        }
    }
}

fn process_enemy_death(
    mut commands: Commands,
    enemies_q: Query<(Entity, &Health), With<Enemy>>,
    mut bar_map: ResMut<EnemyHpBarMap>,
    children_q: Query<&Children>,
) {
    for (e, hp) in &enemies_q {
        if hp.current <= 0.0 {
            if let Some(&bar_e) = bar_map.0.get(&e) {
                despawn_with_children(&mut commands, &children_q, bar_e);
                bar_map.0.remove(&e);
            }
            // 递归删除敌人（如果敌人有子实体）
            despawn_with_children(&mut commands, &children_q, e);
        }
    }
}