use bevy::prelude::*;
use rand::prelude::*;
use std::collections::{HashMap, HashSet, VecDeque};

use crate::dialogue::DialogueNpcCollider;
use crate::health::{Health, PlayerHitIFrames, try_damage_player};
use crate::ldtk_collision::WallColliders;
use crate::movement::{Player, PlayerHitbox};
use crate::skills::ParseableSkill;
use crate::skills_pool::SkillPool;
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
        Self {
            half: Vec2::new(10.0, 10.0),
        }
    }
}

#[derive(Resource)]
struct EnemySpawnTimer(pub Timer);

impl Default for EnemySpawnTimer {
    fn default() -> Self {
        Self(Timer::from_seconds(1.0, TimerMode::Repeating))
    }
}

#[derive(Resource)]
struct EnemyNavFlow {
    timer: Timer,
    cell_size: f32,
    origin: Vec2,
    bounds_min: IVec2,
    bounds_max: IVec2,
    dist_to_player: HashMap<IVec2, u16>,
    valid: bool,
}

impl Default for EnemyNavFlow {
    fn default() -> Self {
        Self {
            timer: Timer::from_seconds(0.12, TimerMode::Repeating),
            cell_size: 16.0,
            origin: Vec2::ZERO,
            bounds_min: IVec2::ZERO,
            bounds_max: IVec2::ZERO,
            dist_to_player: HashMap::new(),
            valid: false,
        }
    }
}

impl EnemyNavFlow {
    fn world_to_cell(&self, pos: Vec2) -> IVec2 {
        let v = (pos - self.origin) / self.cell_size.max(1.0);
        IVec2::new(v.x.round() as i32, v.y.round() as i32)
    }

    fn cell_to_world(&self, cell: IVec2) -> Vec2 {
        self.origin + cell.as_vec2() * self.cell_size
    }

    fn next_target(&self, pos: Vec2, fallback_player_pos: Vec2) -> Option<Vec2> {
        if !self.valid || self.dist_to_player.is_empty() {
            return None;
        }

        let cell = self.world_to_cell(pos);
        let mut best_cell = None;
        let mut best_dist = u16::MAX;

        for candidate in cells_near(cell, 2) {
            if let Some(&d) = self.dist_to_player.get(&candidate) {
                if d < best_dist {
                    best_dist = d;
                    best_cell = Some(candidate);
                }
            }
        }

        let current = best_cell?;
        if best_dist == 0 {
            return Some(fallback_player_pos);
        }

        let mut next = current;
        let mut next_dist = best_dist;
        for n in four_neighbors(current) {
            if let Some(&d) = self.dist_to_player.get(&n) {
                if d < next_dist {
                    next_dist = d;
                    next = n;
                }
            }
        }

        if next == current {
            Some(self.cell_to_world(current))
        } else {
            Some(self.cell_to_world(next))
        }
    }
}

pub struct EnemyPlugin;

impl Plugin for EnemyPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<EnemySpawnTimer>()
            .init_resource::<EnemyNavFlow>()
            .add_systems(
                Update,
                (
                    spawn_enemies_periodically.run_if(in_state(GameState::InGame)),
                    rebuild_enemy_nav_flow
                        .run_if(in_state(GameState::InGame))
                        .before(move_enemies_towards_player),
                    move_enemies_towards_player.run_if(in_state(GameState::InGame)),
                    damage_player_on_contact.run_if(in_state(GameState::InGame)),
                ),
            );
    }
}

fn four_neighbors(cell: IVec2) -> [IVec2; 4] {
    [
        cell + IVec2::X,
        cell - IVec2::X,
        cell + IVec2::Y,
        cell - IVec2::Y,
    ]
}

fn cells_near(center: IVec2, radius: i32) -> impl Iterator<Item = IVec2> {
    (-radius..=radius)
        .flat_map(move |dy| (-radius..=radius).map(move |dx| center + IVec2::new(dx, dy)))
}

fn rebuild_enemy_nav_flow(
    time: Res<Time>,
    walls: Res<WallColliders>,
    player_q: Query<&Transform, With<Player>>,
    enemy_q: Query<&Transform, With<Enemy>>,
    mut nav: ResMut<EnemyNavFlow>,
) {
    nav.timer.tick(time.delta());
    if !nav.timer.just_finished() && !walls.is_changed() {
        return;
    }

    nav.dist_to_player.clear();
    nav.valid = false;

    let Ok(player_tf) = player_q.single() else {
        return;
    };

    let player_pos = player_tf.translation.truncate();
    if walls.aabbs.is_empty() {
        return;
    }

    let cell_size = (walls.half_size.x * 2.0).max(1.0);
    let anchor = walls.aabbs[0].0;
    let origin = Vec2::new(
        anchor.x.rem_euclid(cell_size),
        anchor.y.rem_euclid(cell_size),
    );

    nav.cell_size = cell_size;
    nav.origin = origin;

    let to_cell = |pos: Vec2| {
        let v = (pos - origin) / cell_size;
        IVec2::new(v.x.round() as i32, v.y.round() as i32)
    };

    let mut blocked = HashSet::with_capacity(walls.aabbs.len());
    let mut min = IVec2::splat(i32::MAX);
    let mut max = IVec2::splat(i32::MIN);

    for (center, _) in &walls.aabbs {
        let c = to_cell(*center);
        blocked.insert(c);
        min = min.min(c);
        max = max.max(c);
    }

    let player_cell = to_cell(player_pos);
    min = min.min(player_cell);
    max = max.max(player_cell);
    for tf in &enemy_q {
        let c = to_cell(tf.translation.truncate());
        min = min.min(c);
        max = max.max(c);
    }

    let margin = 24;
    min -= IVec2::splat(margin);
    max += IVec2::splat(margin);
    nav.bounds_min = min;
    nav.bounds_max = max;

    let mut start = None;
    for c in cells_near(player_cell, 2) {
        if c.x < min.x || c.y < min.y || c.x > max.x || c.y > max.y {
            continue;
        }
        if !blocked.contains(&c) {
            start = Some(c);
            break;
        }
    }
    let Some(start_cell) = start else {
        return;
    };

    let mut queue = VecDeque::new();
    nav.dist_to_player.insert(start_cell, 0);
    queue.push_back(start_cell);

    const MAX_NAV_NODES: usize = 20_000;
    while let Some(cell) = queue.pop_front() {
        let cur_dist = *nav.dist_to_player.get(&cell).unwrap_or(&0);
        for next in four_neighbors(cell) {
            if next.x < min.x || next.y < min.y || next.x > max.x || next.y > max.y {
                continue;
            }
            if blocked.contains(&next) || nav.dist_to_player.contains_key(&next) {
                continue;
            }
            nav.dist_to_player.insert(next, cur_dist.saturating_add(1));
            if nav.dist_to_player.len() >= MAX_NAV_NODES {
                nav.valid = true;
                return;
            }
            queue.push_back(next);
        }
    }

    nav.valid = !nav.dist_to_player.is_empty();
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

fn clamp_to_map_bounds(pos: Vec2, half: Vec2, walls: &WallColliders) -> Vec2 {
    let Some((min, max)) = walls.bounds else {
        return pos;
    };

    Vec2::new(
        pos.x.clamp(min.x + half.x, max.x - half.x),
        pos.y.clamp(min.y + half.y, max.y - half.y),
    )
}

fn spawn_enemies_periodically(
    mut commands: Commands,
    time: Res<Time>,
    mut timer: ResMut<EnemySpawnTimer>,
    walls: Res<WallColliders>,
    player_q: Query<&Transform, With<Player>>,
    npc_q: Query<(&Transform, &DialogueNpcCollider)>,
    alive_enemies: Query<&Health, With<Enemy>>,
    asset_server: Res<AssetServer>,
    mut skill_pool: ResMut<SkillPool>,
) {
    timer.0.tick(time.delta());
    if !timer.0.just_finished() {
        return;
    }

    let alive_count = alive_enemies.iter().filter(|hp| hp.current > 0.0).count();
    if alive_count >= 4 {
        return;
    }

    let Ok(player_tf) = player_q.single() else {
        return;
    };
    let player_pos = player_tf.translation.truncate();

    let enemy_half = EnemyHitbox::default().half;
    let probe_half = Vec2::splat(0.5);

    let jitter_x = (walls.half_size.x - enemy_half.x - 1.0).max(0.0);
    let jitter_y = (walls.half_size.y - enemy_half.y - 1.0).max(0.0);

    let mut rng = thread_rng();
    let mut spawn_pos = None;

    for _ in 0..64 {
        let pos = if walls.walkables.is_empty() {
            let angle = rng.gen_range(0.0..std::f32::consts::TAU);
            let dist = rng.gen_range(120.0..=240.0);
            player_pos + Vec2::from_angle(angle) * dist
        } else {
            let base = walls.walkables[rng.gen_range(0..walls.walkables.len())];
            let jitter = Vec2::new(
                rng.gen_range(-jitter_x..=jitter_x),
                rng.gen_range(-jitter_y..=jitter_y),
            );
            base + jitter
        };

        if pos.distance(player_pos) < 80.0 {
            continue;
        }

        let mut in_wall = false;
        for (c, half) in walls.aabbs.iter() {
            if aabb_intersects(pos, enemy_half, *c, *half)
                || aabb_intersects(pos, probe_half, *c, *half)
            {
                in_wall = true;
                break;
            }
        }
        if in_wall {
            continue;
        }

        let overlaps_npc = npc_q.iter().any(|(tf, collider)| {
            aabb_intersects(pos, enemy_half, tf.translation.truncate(), collider.half)
        });
        if overlaps_npc {
            continue;
        }

        spawn_pos = Some(pos);
        break;
    }

    let Some(pos) = spawn_pos else {
        return;
    };

    let texture: Handle<Image> = asset_server.load("enemy.png");
    let mut sprite = Sprite::from_image(texture);
    sprite.custom_size = Some(Vec2::splat(28.0));
    let parse_skill = skill_pool.next_non_dash();

    commands.spawn((
        sprite,
        Transform::from_translation(pos.extend(10.0)),
        Enemy,
        ParseableSkill { skill: parse_skill },
        EnemyAggro(false),
        EnemyHitbox { half: enemy_half },
        EnemySpeed(70.0),
        EnemyDamage(8.0),
        Health {
            current: 40.0,
            max: 40.0,
        },
    ));
}

fn move_enemies_towards_player(
    time: Res<Time>,
    walls: Res<WallColliders>,
    nav: Res<EnemyNavFlow>,
    player_q: Query<&Transform, (With<Player>, Without<Enemy>)>,
    mut enemy_q: Query<
        (&mut Transform, &EnemySpeed, &EnemyHitbox, &EnemyAggro),
        (With<Enemy>, Without<Player>),
    >,
) {
    let Ok(player_tf) = player_q.single() else {
        return;
    };
    let ppos = player_tf.translation.truncate();
    let dt = time.delta_secs();

    for (mut tf, speed, hitbox, aggro) in enemy_q.iter_mut() {
        if !aggro.0 {
            continue;
        }

        let pos = tf.translation.truncate();
        let dir = nav
            .next_target(pos, ppos)
            .map(|target| (target - pos).normalize_or_zero())
            .filter(|d| *d != Vec2::ZERO)
            .unwrap_or_else(|| (ppos - pos).normalize_or_zero());
        let delta = dir * speed.0 * dt;

        let mut new_pos = move_with_walls(pos, delta, hitbox.half, &walls.aabbs);
        new_pos = clamp_to_map_bounds(new_pos, hitbox.half, &walls);
        tf.translation.x = new_pos.x;
        tf.translation.y = new_pos.y;
    }
}

fn damage_player_on_contact(
    mut player_q: Query<
        (
            &mut Health,
            &mut PlayerHitIFrames,
            &Transform,
            &PlayerHitbox,
        ),
        (With<Player>, Without<Enemy>),
    >,
    enemies_q: Query<
        (&Transform, &EnemyDamage, &EnemyHitbox, &EnemyAggro),
        (With<Enemy>, Without<Player>),
    >,
) {
    let Ok((mut player_hp, mut iframes, player_tf, player_hitbox)) = player_q.single_mut() else {
        return;
    };
    let ppos = player_tf.translation.truncate();

    for (tf, dmg, enemy_hitbox, aggro) in enemies_q.iter() {
        if !aggro.0 {
            continue;
        }

        let epos = tf.translation.truncate();
        if aabb_intersects(ppos, player_hitbox.half, epos, enemy_hitbox.half) {
            if try_damage_player(&mut player_hp, &mut iframes, dmg.0) {
                break;
            }
        }
    }
}
