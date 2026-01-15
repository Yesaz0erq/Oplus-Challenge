use bevy::prelude::*;
use rand::prelude::*;

use crate::health::Health;
use crate::ldtk_collision::WallColliders;
use crate::movement::{Player, PlayerHitbox};
use crate::state::GameState;

#[derive(Component)]
pub struct Enemy;

#[derive(Component)]
pub struct EnemySpeed(pub f32);

#[derive(Component)]
pub struct EnemyDamage(pub f32);

#[derive(Component, Default)]
pub struct EnemyAggro(pub bool);

#[derive(Component, Clone, Copy)]
pub struct EnemyHitbox {
    pub half: Vec2,
}

impl Default for EnemyHitbox {
    fn default() -> Self {
        Self { half: Vec2::splat(14.0) }
    }
}

#[derive(Resource)]
struct EnemySpawnTimer(pub Timer);

impl Default for EnemySpawnTimer {
    fn default() -> Self {
        Self(Timer::from_seconds(1.0, TimerMode::Repeating))
    }
}

pub struct EnemyPlugin;

impl Plugin for EnemyPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<EnemySpawnTimer>().add_systems(
            Update,
            (
                spawn_enemies_periodically.run_if(in_state(GameState::InGame)),
                move_enemies_towards_player.run_if(in_state(GameState::InGame)),
                damage_player_on_contact.run_if(in_state(GameState::InGame)),
            ),
        );
    }
}

fn aabb_intersects(a_center: Vec2, a_half: Vec2, b_center: Vec2, b_half: Vec2) -> bool {
    let d = a_center - b_center;
    d.x.abs() < (a_half.x + b_half.x) && d.y.abs() < (a_half.y + b_half.y)
}

fn move_with_walls(start: Vec2, delta: Vec2, half: Vec2, walls: &[(Vec2, Vec2)]) -> Vec2 {
    if walls.is_empty() || delta == Vec2::ZERO {
        return start + delta;
    }

    let mut pos = start;

    pos.x += delta.x;
    for (c, wall_half) in walls.iter() {
        if aabb_intersects(pos, half, *c, *wall_half) {
            if delta.x > 0.0 {
                pos.x = c.x - wall_half.x - half.x;
            } else if delta.x < 0.0 {
                pos.x = c.x + wall_half.x + half.x;
            }
        }
    }

    pos.y += delta.y;
    for (c, wall_half) in walls.iter() {
        if aabb_intersects(pos, half, *c, *wall_half) {
            if delta.y > 0.0 {
                pos.y = c.y - wall_half.y - half.y;
            } else if delta.y < 0.0 {
                pos.y = c.y + wall_half.y + half.y;
            }
        }
    }

    pos
}

fn spawn_enemies_periodically(
    mut commands: Commands,
    time: Res<Time>,
    mut timer: ResMut<EnemySpawnTimer>,
    walls: Res<WallColliders>,
    player_q: Query<&Transform, With<Player>>,
    alive_enemies: Query<&Health, With<Enemy>>,
    asset_server: Res<AssetServer>,
) {
    timer.0.tick(time.delta());
    if !timer.0.just_finished() {
        return;
    }

    let alive_count = alive_enemies.iter().filter(|hp| hp.current > 0.0).count();
    if alive_count >= 4 {
        return;
    }

    if walls.walkables.is_empty() {
        return;
    }

    let Ok(player_tf) = player_q.single() else { return; };
    let player_pos = player_tf.translation.truncate();

    let enemy_half = EnemyHitbox::default().half;

    let jitter_x = (walls.half_size.x - enemy_half.x - 1.0).max(0.0);
    let jitter_y = (walls.half_size.y - enemy_half.y - 1.0).max(0.0);

    let mut rng = thread_rng();
    let mut spawn_pos = None;

    for _ in 0..64 {
        let base = walls.walkables[rng.gen_range(0..walls.walkables.len())];
        let jitter = Vec2::new(rng.gen_range(-jitter_x..=jitter_x), rng.gen_range(-jitter_y..=jitter_y));
        let pos = base + jitter;

        if pos.distance(player_pos) < 80.0 {
            continue;
        }

        let mut overlaps_wall = false;
        for (c, half) in walls.aabbs.iter() {
            if aabb_intersects(pos, enemy_half, *c, *half) {
                overlaps_wall = true;
                break;
            }
        }
        if overlaps_wall {
            continue;
        }

        spawn_pos = Some(pos);
        break;
    }

    let Some(pos) = spawn_pos else { return; };

    let texture: Handle<Image> = asset_server.load("enemy.png");
    let mut sprite = Sprite::from_image(texture);
    sprite.custom_size = Some(Vec2::splat(28.0));

    commands.spawn((
        sprite,
        Transform::from_translation(pos.extend(10.0)),
        Enemy,
        EnemyAggro(false),
        EnemyHitbox { half: enemy_half },
        EnemySpeed(70.0),
        EnemyDamage(8.0),
        Health { current: 40.0, max: 40.0 },
    ));
}

fn move_enemies_towards_player(
    time: Res<Time>,
    walls: Res<WallColliders>,
    player_q: Query<&Transform, (With<Player>, Without<Enemy>)>,
    mut enemy_q: Query<(&mut Transform, &EnemySpeed, &EnemyHitbox, &EnemyAggro), (With<Enemy>, Without<Player>)>,
) {
    let Ok(player_tf) = player_q.single() else { return; };
    let ppos = player_tf.translation.truncate();
    let dt = time.delta_secs();

    for (mut tf, speed, hitbox, aggro) in enemy_q.iter_mut() {
        if !aggro.0 {
            continue;
        }

        let pos = tf.translation.truncate();
        let dir = (ppos - pos).normalize_or_zero();
        let delta = dir * speed.0 * dt;

        let new_pos = move_with_walls(pos, delta, hitbox.half, &walls.aabbs);
        tf.translation.x = new_pos.x;
        tf.translation.y = new_pos.y;
    }
}

fn damage_player_on_contact(
    mut player_q: Query<(&mut Health, &Transform, &PlayerHitbox), (With<Player>, Without<Enemy>)>,
    enemies_q: Query<(&Transform, &EnemyDamage, &EnemyHitbox, &EnemyAggro), (With<Enemy>, Without<Player>)>,
) {
    let Ok((mut player_hp, player_tf, player_hitbox)) = player_q.single_mut() else { return; };
    let ppos = player_tf.translation.truncate();

    for (tf, dmg, enemy_hitbox, aggro) in enemies_q.iter() {
        if !aggro.0 {
            continue;
        }

        let epos = tf.translation.truncate();
        if aabb_intersects(ppos, player_hitbox.half, epos, enemy_hitbox.half) {
            player_hp.current -= dmg.0;
        }
    }
}
