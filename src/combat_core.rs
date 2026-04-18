use bevy::ecs::hierarchy::ChildOf;
use bevy::prelude::*;
use std::collections::{HashMap, HashSet};

use crate::enemy::{Enemy, EnemyAggro};
use crate::health::{Health, PlayerHitIFrames, try_damage_player};
use crate::ldtk_collision::WallColliders;
use crate::movement::Player;
use crate::state::GameState;

const HP_BAR_W: f32 = 28.0;
const HP_BAR_H: f32 = 4.0;
const HP_BAR_Y_PAD: f32 = 8.0;

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
                (
                    update_projectiles,
                    update_slash_vfx,
                    sync_enemy_hp_bars,
                    update_enemy_hp_bars.after(sync_enemy_hp_bars),
                    process_enemy_death,
                )
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
    pub hit_radius: f32,
    pub collides_with_walls: bool,
}

#[derive(Component)]
pub struct SlashVfx {
    pub timer: Timer,
}

#[derive(Component)]
pub struct EnemyHpBar {
    pub owner: Entity,
    pub ratio: f32,
    pub fill: Entity,
}

#[derive(Component)]
pub struct EnemyHpBarFill;

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
    spawn_projectile_custom(
        commands,
        pool,
        origin,
        dir,
        speed,
        lifetime,
        damage,
        from_player,
        Vec2::splat(8.0),
        Color::srgb(1.0, 0.2, 0.2),
        12.0,
        false,
    );
}

#[allow(clippy::too_many_arguments)]
pub fn spawn_projectile_custom(
    commands: &mut Commands,
    pool: Option<&mut ProjectilePool>,
    origin: Vec2,
    dir: Vec2,
    speed: f32,
    lifetime: f32,
    damage: f32,
    from_player: bool,
    sprite_size: Vec2,
    sprite_color: Color,
    hit_radius: f32,
    collides_with_walls: bool,
) {
    let forward = dir.normalize_or_zero();
    if forward == Vec2::ZERO {
        return;
    }

    let mut sprite = Sprite::default();
    sprite.color = sprite_color;
    sprite.custom_size = Some(sprite_size);

    if let Some(pool) = pool {
        if let Some(ent) = pool.free.pop() {
            commands.entity(ent).insert((
                Projectile {
                    direction: forward,
                    speed,
                    lifetime,
                    damage,
                    from_player,
                    hit_radius,
                    collides_with_walls,
                },
                sprite,
                Transform::from_xyz(origin.x, origin.y, 10.0),
            ));
            return;
        }
    }

    commands.spawn((
        Projectile {
            direction: forward,
            speed,
            lifetime,
            damage,
            from_player,
            hit_radius,
            collides_with_walls,
        },
        sprite,
        Transform::from_xyz(origin.x, origin.y, 10.0),
    ));
}

pub fn spawn_fireball_skill_projectile(
    commands: &mut Commands,
    pool: Option<&mut ProjectilePool>,
    origin: Vec2,
    dir: Vec2,
    damage: f32,
) {
    spawn_projectile_custom(
        commands,
        pool,
        origin,
        dir,
        430.0,
        1.8,
        damage,
        true,
        Vec2::splat(18.0),
        Color::srgb(1.0, 0.48, 0.12),
        18.0,
        true,
    );
}

pub fn skill_slash(
    origin: Vec2,
    dir: Vec2,
    enemies_q: &mut Query<(Entity, &Transform, &mut Health, &mut EnemyAggro), With<Enemy>>,
    damage: f32,
) {
    let length: f32 = 260.0;
    let width: f32 = 100.0;
    const EPS: f32 = 6.0;

    let forward = {
        let f = dir.normalize_or_zero();
        if f == Vec2::ZERO { Vec2::Y } else { f }
    };
    let right = Vec2::new(-forward.y, forward.x);

    for (_entity, tf, mut hp, mut aggro) in enemies_q.iter_mut() {
        let to_target = tf.translation.truncate() - origin;
        let d_forward = to_target.dot(forward);
        let d_side = to_target.dot(right);

        if d_forward >= -EPS && d_forward <= length + EPS && d_side.abs() <= (width * 0.5 + EPS) {
            hp.current -= damage;
            aggro.0 = true;
        }
    }
}

pub fn skill_slash_on_player(
    origin: Vec2,
    dir: Vec2,
    player_pos: Vec2,
    player_hp: &mut Health,
    player_iframes: &mut PlayerHitIFrames,
) {
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
        let _ = try_damage_player(player_hp, player_iframes, damage);
    }
}

pub fn spawn_slash_vfx(
    commands: &mut Commands,
    pool: Option<&mut VfxPool>,
    origin: Vec2,
    dir: Vec2,
) {
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

    let tf = Transform {
        translation: center.extend(15.0),
        rotation: Quat::from_rotation_z(angle),
        ..Default::default()
    };
    let vfx = SlashVfx {
        timer: Timer::from_seconds(0.2, TimerMode::Once),
    };

    if let Some(pool) = pool {
        if let Some(ent) = pool.free.pop() {
            commands.entity(ent).insert((sprite, tf, vfx));
            return;
        }
    }

    commands.spawn((sprite, tf, vfx));
}

pub fn skill_light_wave(
    origin: Vec2,
    dir: Vec2,
    enemies_q: &mut Query<(Entity, &Transform, &mut Health, &mut EnemyAggro), With<Enemy>>,
    damage: f32,
) {
    let length: f32 = 520.0;
    let width: f32 = 42.0;
    const EPS: f32 = 4.0;

    let forward = {
        let f = dir.normalize_or_zero();
        if f == Vec2::ZERO { Vec2::Y } else { f }
    };
    let right = Vec2::new(-forward.y, forward.x);

    for (_entity, tf, mut hp, mut aggro) in enemies_q.iter_mut() {
        let to_target = tf.translation.truncate() - origin;
        let d_forward = to_target.dot(forward);
        let d_side = to_target.dot(right);

        if d_forward >= -EPS && d_forward <= length + EPS && d_side.abs() <= (width * 0.5 + EPS) {
            hp.current -= damage;
            aggro.0 = true;
        }
    }
}

pub fn spawn_light_wave_vfx(
    commands: &mut Commands,
    pool: Option<&mut VfxPool>,
    origin: Vec2,
    dir: Vec2,
) {
    let forward = dir.normalize_or_zero();
    if forward == Vec2::ZERO {
        return;
    }

    let length: f32 = 520.0;
    let width: f32 = 28.0;

    let mut sprite = Sprite::default();
    sprite.color = Color::srgba(0.45, 0.95, 1.0, 0.75);
    sprite.custom_size = Some(Vec2::new(length, width));

    let center = origin + forward * (length * 0.5);
    let angle = forward.y.atan2(forward.x);

    let tf = Transform {
        translation: center.extend(15.0),
        rotation: Quat::from_rotation_z(angle),
        ..Default::default()
    };
    let vfx = SlashVfx {
        timer: Timer::from_seconds(0.14, TimerMode::Once),
    };

    if let Some(pool) = pool {
        if let Some(ent) = pool.free.pop() {
            commands.entity(ent).insert((sprite, tf, vfx));
            return;
        }
    }

    commands.spawn((sprite, tf, vfx));
}

fn update_slash_vfx(
    time: Res<Time>,
    mut commands: Commands,
    mut q: Query<(Entity, &mut SlashVfx)>,
    mut vfx_pool: ResMut<VfxPool>,
) {
    let dt = time.delta();
    for (entity, mut vfx) in q.iter_mut() {
        vfx.timer.tick(dt);
        if vfx.timer.is_finished() {
            commands.entity(entity).remove::<(SlashVfx, Sprite)>();
            vfx_pool.free.push(entity);
        }
    }
}

fn update_projectiles(
    time: Res<Time>,
    mut commands: Commands,
    mut proj_q: Query<(Entity, &mut Projectile, &mut Transform), With<Projectile>>,
    walls: Res<WallColliders>,
    mut enemies_q: Query<
        (Entity, &Transform, &mut Health, &mut EnemyAggro),
        (With<Enemy>, Without<Projectile>, Without<Player>),
    >,

    mut player_q: Query<
        (&Transform, &mut Health, &mut PlayerHitIFrames),
        (With<Player>, Without<Projectile>, Without<Enemy>),
    >,
    mut pool: ResMut<ProjectilePool>,
) {
    let dt = time.delta_secs();

    for (proj_entity, mut proj, mut proj_tf) in proj_q.iter_mut() {
        proj.lifetime -= dt;
        if proj.lifetime <= 0.0 {
            commands.entity(proj_entity).remove::<Projectile>();
            pool.free.push(proj_entity);
            continue;
        }

        let delta = proj.direction * proj.speed * dt;
        proj_tf.translation.x += delta.x;
        proj_tf.translation.y += delta.y;

        if proj.collides_with_walls
            && walls.aabbs.iter().any(|(c, half)| {
                aabb_intersects(
                    proj_tf.translation.truncate(),
                    Vec2::splat(proj.hit_radius),
                    *c,
                    *half,
                )
            })
        {
            commands.entity(proj_entity).remove::<Projectile>();
            pool.free.push(proj_entity);
            continue;
        }

        if proj.from_player {
            let mut hit = false;
            for (_entity, tf, mut hp, mut aggro) in enemies_q.iter_mut() {
                let dist = tf
                    .translation
                    .truncate()
                    .distance(proj_tf.translation.truncate());
                if dist <= proj.hit_radius {
                    hp.current -= proj.damage;
                    aggro.0 = true;
                    hit = true;
                    break;
                }
            }

            if hit {
                commands.entity(proj_entity).remove::<Projectile>();
                pool.free.push(proj_entity);
            }
        } else if let Ok((player_tf, mut hp, mut iframes)) = player_q.single_mut() {
            let dist = player_tf
                .translation
                .truncate()
                .distance(proj_tf.translation.truncate());
            if dist <= proj.hit_radius {
                let _ = try_damage_player(&mut hp, &mut iframes, proj.damage);
                commands.entity(proj_entity).remove::<Projectile>();
                pool.free.push(proj_entity);
            }
        }
    }
}

fn aabb_intersects(a_center: Vec2, a_half: Vec2, b_center: Vec2, b_half: Vec2) -> bool {
    let d = a_center - b_center;
    d.x.abs() < (a_half.x + b_half.x) && d.y.abs() < (a_half.y + b_half.y)
}

fn sync_enemy_hp_bars(
    mut commands: Commands,
    enemies_q: Query<(Entity, &Health, Option<&Sprite>), With<Enemy>>,
    mut bar_map: ResMut<EnemyHpBarMap>,
    bars_q: Query<&EnemyHpBar>,
) {
    let mut seen = HashSet::new();

    for (enemy_e, health, sprite) in enemies_q.iter() {
        if health.current <= 0.0 {
            continue;
        }

        seen.insert(enemy_e);

        if bar_map.0.contains_key(&enemy_e) {
            continue;
        }

        let ratio = (health.current / health.max).clamp(0.0, 1.0);
        let y = sprite
            .and_then(|s| s.custom_size)
            .map(|sz| sz.y * 0.5 + HP_BAR_Y_PAD)
            .unwrap_or(40.0);

        let bg = commands
            .spawn((
                ChildOf(enemy_e),
                Sprite {
                    color: Color::srgba(0.0, 0.0, 0.0, 0.7),
                    custom_size: Some(Vec2::new(HP_BAR_W, HP_BAR_H)),
                    ..default()
                },
                Transform::from_xyz(0.0, y, 100.0),
            ))
            .id();

        let fill_w = HP_BAR_W * ratio;
        let fill_x = -(HP_BAR_W - fill_w) * 0.5;

        let fill = commands
            .spawn((
                ChildOf(enemy_e),
                EnemyHpBarFill,
                Sprite {
                    color: Color::srgba(0.2, 0.9, 0.2, 0.9),
                    custom_size: Some(Vec2::new(fill_w, HP_BAR_H)),
                    ..default()
                },
                Transform::from_xyz(fill_x, y, 101.0),
            ))
            .id();

        commands.entity(bg).insert(EnemyHpBar {
            owner: enemy_e,
            ratio,
            fill,
        });

        bar_map.0.insert(enemy_e, bg);
    }

    let to_remove: Vec<(Entity, Entity)> = bar_map
        .0
        .iter()
        .filter(|(enemy, _)| !seen.contains(enemy))
        .map(|(enemy, bar)| (*enemy, *bar))
        .collect();

    for (enemy, bar_ent) in to_remove {
        bar_map.0.remove(&enemy);

        if let Ok(bar) = bars_q.get(bar_ent) {
            commands.entity(bar.fill).try_despawn();
        }
        commands.entity(bar_ent).try_despawn();
    }
}

fn update_enemy_hp_bars(
    enemies_q: Query<(&Health, Option<&Sprite>), With<Enemy>>,
    mut bars_q: Query<(&mut EnemyHpBar, &mut Transform)>,
    mut fill_q: Query<
        (&mut Sprite, &mut Transform),
        (With<EnemyHpBarFill>, Without<EnemyHpBar>, Without<Enemy>),
    >,
) {
    for (mut bar, mut bg_tf) in bars_q.iter_mut() {
        let Ok((hp, sprite)) = enemies_q.get(bar.owner) else {
            continue;
        };

        let ratio = (hp.current / hp.max).clamp(0.0, 1.0);

        let y = sprite
            .and_then(|s| s.custom_size)
            .map(|sz| sz.y * 0.5 + HP_BAR_Y_PAD)
            .unwrap_or(bg_tf.translation.y);

        bg_tf.translation.y = y;

        if let Ok((mut fill_sprite, mut fill_tf)) = fill_q.get_mut(bar.fill) {
            fill_tf.translation.y = y;

            if (ratio - bar.ratio).abs() > 0.001 {
                bar.ratio = ratio;

                let fill_w = HP_BAR_W * ratio;
                fill_sprite.custom_size = Some(Vec2::new(fill_w, HP_BAR_H));
                fill_tf.translation.x = -(HP_BAR_W - fill_w) * 0.5;
            }
        }
    }
}

fn process_enemy_death(mut bar_map: ResMut<EnemyHpBarMap>, enemies_q: Query<Entity, With<Enemy>>) {
    let existing: HashSet<Entity> = enemies_q.iter().collect();
    bar_map.0.retain(|enemy, _| existing.contains(enemy));
}
