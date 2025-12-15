use bevy::input::mouse::MouseButton;
use bevy::prelude::*;
use bevy::ecs::system::Single;
use bevy::window::PrimaryWindow;
use std::collections::{HashMap, HashSet};

use crate::equipment::{EquipmentSet, WeaponKind};
use crate::enemy::Enemy;
use crate::health::Health;
use crate::input::MovementInput;
use crate::movement::Player;
use crate::state::GameState;

/// 战斗插件：普通攻击 + Slash 技能 + 弹幕 + 敌人血条
pub struct CombatPlugin;

/// 攻击状态：管理普攻与技能冷却
#[derive(Component, Default)]
pub struct AttackState {
    /// 普通攻击冷却（秒）
    pub basic_cooldown: f32,
    /// Slash 技能冷却
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
    pub ratio: f32,
}

/// 血条填充节点标记
#[derive(Component)]
pub struct EnemyHpBarFill;

/// 资源：敌人 -> 血条实体 映射
#[derive(Resource, Default)]
pub struct EnemyHpBarMap(pub HashMap<Entity, Entity>);

#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone)]
pub struct CombatSet;

impl Plugin for CombatPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<EnemyHpBarMap>();

        // ✅ 把 run_if 放在 Set 上（避免 tuple.run_if 的坑）
        app.configure_sets(Update, CombatSet.run_if(in_state(GameState::InGame)));

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
                .in_set(CombatSet),
        );
    }
}

/// 递归删除实体及其子节点（用于删除 UI 根节点等）
/// 兼容 Bevy 0.17：Children::iter() 返回 Entity（按值）
pub fn despawn_with_children(commands: &mut Commands, children_q: &Query<&Children>, entity: Entity) {
    if let Ok(children) = children_q.get(entity) {
        for child in children.iter() {
            despawn_with_children(commands, children_q, child);
        }
    }
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
/// - 远程：发射子弹（✅ 改为朝鼠标位置瞄准）
fn handle_basic_attack(
    mouse: Res<ButtonInput<MouseButton>>,
    movement: Res<MovementInput>,
    window: Single<&Window, With<PrimaryWindow>>,
    camera: Single<(&Camera, &GlobalTransform), With<Camera2d>>,
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

    // 默认方向：玩家移动方向（给近战用）
    let mut dir = if movement.0 != Vec2::ZERO {
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
            // ✅ 远程：用鼠标光标位置决定方向
            if let Some(screen_pos) = window.cursor_position() {
                let (cam, cam_global) = *camera;
                // 0.17：viewport_to_world_2d 返回 Result<Vec2, _>
                if let Ok(world_pos) = cam.viewport_to_world_2d(cam_global, screen_pos) {
                    let player_pos = player_tf.translation.truncate();
                    let aim = (world_pos - player_pos).normalize_or_zero();
                    if aim != Vec2::ZERO {
                        dir = aim;
                    }
                }
            }

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
    let length: f32 = 260.0;
    let width: f32 = 100.0;
    let damage: f32 = 60.0;

    const EPS: f32 = 6.0;

    let forward = {
        let f = dir.normalize_or_zero();
        if f == Vec2::ZERO { Vec2::Y } else { f }
    };
    let right = Vec2::new(-forward.y, forward.x);

    for (entity, tf, mut hp) in enemies_q.iter_mut() {
        let to_target = tf.translation.truncate() - origin;
        let d_forward = to_target.dot(forward);
        let d_side = to_target.dot(right);

        if d_forward >= -EPS && d_forward <= length + EPS && d_side.abs() <= (width * 0.5 + EPS) {
            hp.current -= damage;
            info!(
                "skill_slash hit entity {}: -{:.1} hp -> {:.1}",
                entity.index(),
                damage,
                hp.current
            );
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
fn update_slash_vfx(time: Res<Time>, mut commands: Commands, mut q: Query<(Entity, &mut SlashVfx)>) {
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
            commands.entity(entity).despawn();
        }
    }
}

/// 更新子弹：移动 + 命中 + 销毁
fn update_projectiles(
    time: Res<Time>,
    mut commands: Commands,
    mut proj_q: Query<(Entity, &mut Projectile, &mut Transform), Without<Enemy>>,
    mut enemies_q: Query<(Entity, &Transform, &mut Health), (With<Enemy>, Without<Projectile>)>,
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
                let dist = enemy_tf.translation.truncate().distance(tf.translation.truncate());
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

/// 同步敌人血条：
/// - 受伤但没死才显示
/// - 血条 Node 用 Absolute 定位（示例：左上列表）
fn sync_enemy_hp_bars(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    enemies_q: Query<(Entity, &Health), With<Enemy>>,
    mut bar_map: ResMut<EnemyHpBarMap>,
    mut param_set: ParamSet<(
        Query<(&mut Node, &Children), With<EnemyHpBar>>, // p0: 根节点
        Query<&mut Node, With<EnemyHpBarFill>>,         // p1: 填充节点
    )>,
    children_q: Query<&Children>,
) {
    // 1) 收集受伤敌人
    let mut damaged: Vec<(Entity, f32, f32)> = Vec::new();
    for (e, hp) in &enemies_q {
        if hp.current > 0.0 && hp.current < hp.max {
            damaged.push((e, hp.current, hp.max));
        }
    }
    damaged.sort_by_key(|(e, _, _)| e.index());

    let mut keep_set: HashSet<Entity> = HashSet::new();

    // 2) 创建/更新血条
    for (i, (enemy_entity, current, max)) in damaged.iter().enumerate() {
        keep_set.insert(*enemy_entity);

        let bar_e = if let Some(&bar_e) = bar_map.0.get(enemy_entity) {
            bar_e
        } else {
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
                    EnemyHpBar {
                        owner: *enemy_entity,
                        ratio: 1.0,
                    },
                ))
                .with_children(|parent| {
                    parent.spawn((
                        Node {
                            width: Val::Percent(100.0),
                            height: Val::Percent(100.0),
                            ..Default::default()
                        },
                        BackgroundColor(Color::srgba(0.8, 0.0, 0.0, 0.95)),
                        EnemyHpBarFill,
                    ));

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

        let ratio_percent = ((*current / *max).clamp(0.0, 1.0)) * 100.0;

        // ✅ 关键修复：先 let 绑定 p0()/p1()，避免 E0716
        let child_entities: Vec<Entity> = {
            let mut q0 = param_set.p0();
            let Ok((mut bar_node, children)) = q0.get_mut(bar_e) else {
                continue;
            };

            bar_node.top = Val::Px(10.0 + i as f32 * 22.0);
            bar_node.left = Val::Px(20.0);

            children.iter().collect()
        };

        {
            let mut q1 = param_set.p1();
            for child in child_entities {
                if let Ok(mut fill_node) = q1.get_mut(child) {
                    fill_node.width = Val::Percent(ratio_percent);
                    break; // 找到填充条就行
                }
            }
        }
    }

    // 3) 清理不再需要的血条
    let existing: Vec<(Entity, Entity)> = bar_map.0.iter().map(|(k, v)| (*k, *v)).collect();
    for (owner, bar_e) in existing {
        if !keep_set.contains(&owner) {
            bar_map.0.remove(&owner);
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
            despawn_with_children(&mut commands, &children_q, e);
        }
    }
}
