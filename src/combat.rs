use bevy::ecs::system::Single;
use bevy::input::mouse::MouseButton;
use bevy::prelude::*;
use bevy::window::PrimaryWindow;

use crate::combat_core::{spawn_projectile, CombatSet};
use crate::equipment::{EquipmentSet, WeaponKind};
use crate::enemy::Enemy;
use crate::health::Health;
use crate::input::MovementInput;
use crate::movement::Player;
use crate::state::GameState;

#[derive(Component, Default)]
pub struct AttackState {
    pub basic_cooldown: f32,
    pub slash_cooldown: f32,
}

pub struct CombatPlugin;

impl Plugin for CombatPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (ensure_attack_state, tick_attack_state, handle_basic_attack, cleanup_dead_enemies)
                .in_set(CombatSet)
                .run_if(in_state(GameState::InGame)),
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
        state.basic_cooldown = (state.basic_cooldown - dt).max(0.0);
        state.slash_cooldown = (state.slash_cooldown - dt).max(0.0);
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
    if state.basic_cooldown > 0.0 {
        return;
    }

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
                true,
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

fn cleanup_dead_enemies(mut commands: Commands, enemies: Query<(Entity, &Health), With<Enemy>>) {
    for (entity, hp) in &enemies {
        if hp.current <= 0.0 {
            commands.entity(entity).try_despawn();
        }
    }
}