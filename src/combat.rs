use bevy::input::mouse::MouseButton;
use bevy::prelude::*;
use std::collections::HashMap;

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

impl Plugin for CombatPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                ensure_attack_state,
                tick_attack_state,
                handle_basic_attack,
                update_projectiles,
                update_slash_vfx,
            )
                .run_if(in_state(GameState::InGame)),
        );
    }
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
    let length: f32 = 260.0;
    let width: f32 = 80.0;
    let damage: f32 = 60.0;

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
