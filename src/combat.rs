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
use crate::utils::despawn_with_children;

#[derive(Component, Default)]
pub struct AttackState {
    pub basic_cooldown: f32,
    pub slash_cooldown: f32,
}

#[derive(Component)]
pub struct Projectile {
    pub direction: Vec2,
    pub speed: f32,
    pub lifetime: f32,
    pub damage: f32,
    pub from_player: bool,
}

#[derive(Component)]
pub struct SlashVfx {
    pub timer: Timer,
}

#[derive(Component)]
pub struct EnemyHpBar {
    pub owner: Entity,
    pub ratio: f32,
}

#[derive(Component)]
pub struct EnemyHpBarFill;

#[derive(Resource, Default)]
pub struct EnemyHpBarMap(pub HashMap<Entity, Entity>);

#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone)]
pub struct CombatSet;

pub struct CombatPlugin;

impl Plugin for CombatPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<EnemyHpBarMap>();
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

fn ensure_attack_state(mut commands: Commands, query: Query<(Entity, Option<&AttackState>), With<Player>>) {
    for (entity, state) in &query {
        if state.is_none() {
            commands.entity(entity).insert(AttackState::default());
        }
    }
}

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

    let Ok((player_tf, equip, mut state)) = player_q.single_mut() else { return; };
    if state.basic_cooldown > 0.0 { return; }

    let mut dir = if movement.0 != Vec2::ZERO { movement.0.normalize() } else { Vec2::Y };

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
            if let Some(screen_pos) = window.cursor_position() {
                let (cam, cam_global) = *camera;
                if let Ok(world_pos) = cam.viewport_to_world_2d(cam_global, screen_pos) {
                    let player_pos = player_tf.translation.truncate();
                    let aim = (world_pos - player_pos).normalize_or_zero();
                    if aim != Vec2::ZERO { dir = aim; }
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

fn perform_melee_attack(
    origin: Vec2,
    dir: Vec2,
    length: f32,
    width: f32,
    damage: f32,
    enemies_q: &mut Query<(Entity, &Transform, &mut Health), With<Enemy>>,
) {
    let forward = dir.normalize_or_zero();
    if forward == Vec2::ZERO { return; }
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

    for (_entity, tf, mut hp) in enemies_q.iter_mut() {
        let to_target = tf.translation.truncate() - origin;
        let d_forward = to_target.dot(forward);
        let d_side = to_target.dot(right);

        if d_forward >= -EPS && d_forward <= length + EPS && d_side.abs() <= (width * 0.5 + EPS) {
            hp.current -= damage;
            info!("skill_slash hit: -{:.1} hp -> {:.1}", damage, hp.current);
        }
    }
}

pub fn spawn_slash_vfx(commands: &mut Commands, origin: Vec2, dir: Vec2) {
    let forward = dir.normalize_or_zero();
    if forward == Vec2::ZERO { return; }

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
        SlashVfx { timer: Timer::from_seconds(0.2, TimerMode::Once) },
    ));
}

fn update_slash_vfx(time: Res<Time>, mut commands: Commands, mut q: Query<(Entity, &mut SlashVfx)>) {
    let dt = time.delta();
    for (entity, mut vfx) in &mut q {
        vfx.timer.tick(dt);
        if vfx.timer.is_finished() {
            commands.entity(entity).despawn();
        }
    }
}

fn spawn_projectile(
    commands: &mut Commands,
    origin: Vec2,
    dir: Vec2,
    speed: f32,
    lifetime: f32,
    damage: f32,
) {
    let forward = dir.normalize_or_zero();
    if forward == Vec2::ZERO { return; }

    let mut sprite = Sprite::default();
    sprite.color = Color::srgb(1.0, 0.2, 0.2);
    sprite.custom_size = Some(Vec2::splat(8.0));

    commands.spawn((
        Projectile { direction: forward, speed, lifetime, damage, from_player: true },
        sprite,
        Transform::from_xyz(origin.x, origin.y, 10.0),
    ));
}

fn cleanup_dead_enemies(mut commands: Commands, enemies: Query<(Entity, &Health), With<Enemy>>) {
    for (entity, hp) in &enemies {
        if hp.current <= 0.0 {
            commands.entity(entity).despawn();
        }
    }
}

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

fn sync_enemy_hp_bars(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    enemies_q: Query<(Entity, &Health, &Transform), With<Enemy>>,
    mut bar_map: ResMut<EnemyHpBarMap>,
) {
    let mut seen = HashSet::new();

    for (enemy_e, health, tf) in enemies_q.iter() {
        if health.current <= 0.0 { continue; }
        seen.insert(enemy_e);

        if !bar_map.0.contains_key(&enemy_e) {
            let bar_ent = commands.spawn((
                Text::new(format!("{:.0}/{:.0}", health.current, health.max)),
                EnemyHpBar { owner: enemy_e, ratio: health.current / health.max },
                Transform::from_translation(tf.translation + Vec3::new(-20.0, 40.0, 100.0)),
            )).id();

            bar_map.0.insert(enemy_e, bar_ent);
        } else {
            // optionally update existing bar component (left as an exercise)
        }
    }
    
    let to_remove: Vec<(Entity, Entity)> = bar_map
        .0
        .iter()
        .filter(|(enemy, _)| !seen.contains(enemy))
        .map(|(enemy, bar)| (*enemy, *bar))
        .collect();

    for (enemy, bar_ent) in to_remove {
        bar_map.0.remove(&enemy);
        commands.entity(bar_ent).despawn();
    }
}

fn process_enemy_death(mut bar_map: ResMut<EnemyHpBarMap>, enemies_q: Query<Entity, With<Enemy>>) {
    // 简化的清理：移除 map 中不存在的敌人条目（如果需要更复杂逻辑可扩展）
    let existing: HashSet<Entity> = enemies_q.iter().collect();
    bar_map.0.retain(|enemy, _bar| existing.contains(enemy));
}
