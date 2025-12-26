use bevy::prelude::*;
use std::collections::{HashMap, HashSet};

use crate::enemy::Enemy;
use crate::health::Health;
use crate::movement::Player;
use crate::state::GameState;

#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone)]
pub struct CombatSet;

pub struct CombatCorePlugin;

impl Plugin for CombatCorePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<EnemyHpBarMap>()
            .init_resource::<ProjectilePool>()
            .init_resource::<VfxPool>()
            .configure_sets(Update, CombatSet.run_if(in_state(GameState::InGame)))
            .add_systems(
                Update,
                (update_projectiles, update_slash_vfx, sync_enemy_hp_bars, process_enemy_death)
                    .in_set(CombatSet),
            );
    }
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

#[derive(Resource, Default)]
pub struct EnemyHpBarMap(pub HashMap<Entity, Entity>);

#[derive(Resource, Default)]
pub struct ProjectilePool {
    pub free: Vec<Entity>,
}

#[derive(Resource, Default)]
pub struct VfxPool {
    pub free: Vec<Entity>,
}

pub fn spawn_projectile(
    commands: &mut Commands,
    pool: Option<&mut ProjectilePool>,
    origin: Vec2,
    dir: Vec2,
    speed: f32,
    lifetime: f32,
    damage: f32,
    from_player: bool,
) {
    let forward = dir.normalize_or_zero();
    if forward == Vec2::ZERO {
        return;
    }

    let mut sprite = Sprite::default();
    sprite.color = Color::srgb(1.0, 0.2, 0.2);
    sprite.custom_size = Some(Vec2::splat(8.0));

    if let Some(pool) = pool {
        if let Some(ent) = pool.free.pop() {
            commands.entity(ent).insert((
                Projectile { direction: forward, speed, lifetime, damage, from_player },
                sprite,
                Transform::from_xyz(origin.x, origin.y, 10.0),
            ));
            return;
        }
    }

    commands.spawn((
        Projectile { direction: forward, speed, lifetime, damage, from_player },
        sprite,
        Transform::from_xyz(origin.x, origin.y, 10.0),
    ));
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
        }
    }
}

pub fn skill_slash_on_player(origin: Vec2, dir: Vec2, player_pos: Vec2, player_hp: &mut Health) {
    let length: f32 = 160.0;
    let width: f32 = 80.0;
    let damage: f32 = 25.0;

    let forward = dir.normalize_or_zero();
    if forward == Vec2::ZERO {
        return;
    }
    let right = Vec2::new(-forward.y, forward.x);

    let to_target = player_pos - origin;
    let d_forward = to_target.dot(forward);
    let d_side = to_target.dot(right);

    if d_forward >= 0.0 && d_forward <= length && d_side.abs() <= width * 0.5 {
        player_hp.current -= damage;
    }
}

pub fn spawn_slash_vfx(commands: &mut Commands, pool: Option<&mut VfxPool>, origin: Vec2, dir: Vec2) {
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

    if let Some(pool) = pool {
        if let Some(ent) = pool.free.pop() {
            commands.entity(ent).insert((
                sprite,
                Transform {
                    translation: center.extend(15.0),
                    rotation: Quat::from_rotation_z(angle),
                    ..Default::default()
                },
                SlashVfx { timer: Timer::from_seconds(0.2, TimerMode::Once) },
            ));
            return;
        }
    }

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

fn update_slash_vfx(time: Res<Time>, mut commands: Commands, mut q: Query<(Entity, &mut SlashVfx)>, mut vfx_pool: ResMut<VfxPool>) {
    let dt = time.delta();
    for (entity, mut vfx) in &mut q {
        vfx.timer.tick(dt);
        if vfx.timer.is_finished() {
            commands.entity(entity).remove::<SlashVfx>();
            vfx_pool.free.push(entity);
        }
    }
}

fn update_projectiles(
    time: Res<Time>,
    mut commands: Commands,
    mut proj_q: Query<(Entity, &mut Projectile, &mut Transform), With<Projectile>>,
    mut enemies_q: Query<
        (Entity, &Transform, &mut Health),
        (With<Enemy>, Without<Projectile>, Without<Player>),
    >,
    mut player_q: Query<
        (&Transform, &mut Health),
        (With<Player>, Without<Projectile>, Without<Enemy>),
    >,
    mut pool: ResMut<ProjectilePool>,
) {
    let dt = time.delta_secs();

    for (proj_entity, mut proj, mut tf) in &mut proj_q {
        proj.lifetime -= dt;
        if proj.lifetime <= 0.0 {
            commands.entity(proj_entity).remove::<Projectile>();
            pool.free.push(proj_entity);
            continue;
        }

        let delta = proj.direction * proj.speed * dt;
        tf.translation.x += delta.x;
        tf.translation.y += delta.y;

        let hit_radius = 12.0;

        if proj.from_player {
            let mut hit = false;
            for (_enemy_entity, enemy_tf, mut hp) in &mut enemies_q {
                let dist = enemy_tf.translation.truncate().distance(tf.translation.truncate());
                if dist <= hit_radius {
                    hp.current -= proj.damage;
                    hit = true;
                }
            }
            if hit {
                commands.entity(proj_entity).remove::<Projectile>();
                pool.free.push(proj_entity);
            }
        } else {
            if let Ok((player_tf, mut hp)) = player_q.single_mut() {
                let dist = player_tf.translation.truncate().distance(tf.translation.truncate());
                if dist <= hit_radius {
                    hp.current -= proj.damage;
                    commands.entity(proj_entity).remove::<Projectile>();
                    pool.free.push(proj_entity);
                }
            }
        }
    }
}

fn sync_enemy_hp_bars(
    mut commands: Commands,
    enemies_q: Query<(Entity, &Health, &Transform), With<Enemy>>,
    mut bar_map: ResMut<EnemyHpBarMap>,
) {
    let mut seen = HashSet::new();

    for (enemy_e, health, tf) in enemies_q.iter() {
        if health.current <= 0.0 {
            continue;
        }
        seen.insert(enemy_e);

        if !bar_map.0.contains_key(&enemy_e) {
            let bar_ent = commands
                .spawn((
                    Text::new(format!("{:.0}/{:.0}", health.current, health.max)),
                    EnemyHpBar { owner: enemy_e, ratio: health.current / health.max },
                    Transform::from_translation(tf.translation + Vec3::new(-20.0, 40.0, 100.0)),
                ))
                .id();

            bar_map.0.insert(enemy_e, bar_ent);
        } else {
            if let Some(&bar_ent) = bar_map.0.get(&enemy_e) {
                commands.entity(bar_ent).insert(Text::new(format!("{:.0}/{:.0}", health.current, health.max)));
            }
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
        commands.entity(bar_ent).try_despawn();
    }
}

fn process_enemy_death(mut bar_map: ResMut<EnemyHpBarMap>, enemies_q: Query<Entity, With<Enemy>>) {
    let existing: HashSet<Entity> = enemies_q.iter().collect();
    bar_map.0.retain(|enemy, _bar| existing.contains(enemy));
}
