// src/movement.rs
use bevy::input::keyboard::KeyCode;
use bevy::prelude::*;
use bevy_ecs_ldtk::prelude::EntityInstance;

use crate::{
    health::Health, input::MovementInput, ldtk_collision::WallColliders, state::GameState,
};

pub struct MovementPlugin;

#[derive(Component)]
pub struct Player;

#[derive(Component)]
pub struct PlayerCamera;

#[derive(Component)]
pub struct Background;

const PLAYER_SPEED: f32 = 200.0;
const SPRINT_MULTIPLIER: f32 = 1.5;
const DASH_MULTIPLIER: f32 = 3.0;
pub const DASH_DURATION: f32 = 0.4;
pub const DASH_COOLDOWN: f32 = 10.0;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum PlayerDirection {
    Down,
    Left,
    Right,
    Up,
}

impl PlayerDirection {
    pub fn row_index(self) -> usize {
        match self {
            PlayerDirection::Down => 0,
            PlayerDirection::Left => 1,
            PlayerDirection::Right => 2,
            PlayerDirection::Up => 3,
        }
    }

    pub fn as_vec2(self) -> Vec2 {
        match self {
            PlayerDirection::Down => Vec2::new(0.0, -1.0),
            PlayerDirection::Up => Vec2::new(0.0, 1.0),
            PlayerDirection::Left => Vec2::new(-1.0, 0.0),
            PlayerDirection::Right => Vec2::new(1.0, 0.0),
        }
    }
}

#[derive(Component, Debug)]
pub struct PlayerAnimation {
    pub direction: PlayerDirection,
    pub is_moving: bool,
    frame: usize,
    columns: usize,
    rows: usize,
    initialized: bool,
    frame_size: Vec2,
    timer: Timer,
}

impl Default for PlayerAnimation {
    fn default() -> Self {
        Self {
            frame: 0,
            columns: 1,
            rows: 4,
            direction: PlayerDirection::Down,
            initialized: false,
            frame_size: Vec2::ZERO,
            timer: Timer::from_seconds(0.12, TimerMode::Repeating),
            is_moving: false,
        }
    }
}

#[derive(Component, Default, Debug)]
pub struct PlayerDash {
    pub is_dashing: bool,
    pub remaining: f32,
    pub cooldown: f32,
    pub direction: Vec2,
}

#[derive(Component, Clone, Copy, Debug)]
pub struct PlayerHitbox {
    pub half: Vec2,
}

impl Default for PlayerHitbox {
    fn default() -> Self {
        Self {
            half: Vec2::new(1.0, 1.0),
        }
    }
}

fn init_player_animation(
    images: Res<Assets<Image>>,
    mut query: Query<(&mut Sprite, &mut PlayerAnimation), With<Player>>,
) {
    for (mut sprite, mut anim) in &mut query {
        if anim.initialized {
            continue;
        }

        let Some(image) = images.get(&sprite.image) else {
            continue;
        };

        let size = image.size();
        let tex_width = size.x as f32;
        let tex_height = size.y as f32;

        let rows = anim.rows as f32;
        let frame_height = tex_height / rows;
        let frame_width = frame_height;

        let columns = (tex_width / frame_width).floor().max(1.0) as usize;

        anim.columns = columns;
        anim.frame_size = Vec2::new(frame_width, frame_height);
        anim.initialized = true;

        update_sprite_rect(&mut sprite, &anim);
    }
}

fn apply_player_movement(
    time: Res<Time>,
    keyboard: Res<ButtonInput<KeyCode>>,
    movement: Res<MovementInput>,
    walls: Res<WallColliders>,
    mut query: Query<
        (
            &mut Transform,
            &mut PlayerAnimation,
            &mut PlayerDash,
            &PlayerHitbox,
        ),
        With<Player>,
    >,
) {
    let dt = time.delta_secs();
    let Ok((mut transform, mut anim, mut dash, hitbox)) = query.single_mut() else {
        return;
    };

    let input_dir = movement.0;
    let mut move_dir = input_dir;

    if dash.cooldown > 0.0 {
        dash.cooldown = (dash.cooldown - dt).max(0.0);
    }

    if dash.is_dashing {
        dash.remaining -= dt;
        if dash.remaining <= 0.0 {
            dash.is_dashing = false;
        } else {
            move_dir = dash.direction;
        }
    }

    if move_dir != Vec2::ZERO {
        anim.direction = if move_dir.x.abs() > move_dir.y.abs() {
            if move_dir.x > 0.0 {
                PlayerDirection::Right
            } else {
                PlayerDirection::Left
            }
        } else if move_dir.y > 0.0 {
            PlayerDirection::Up
        } else {
            PlayerDirection::Down
        };
    }

    let mut speed = PLAYER_SPEED;
    if dash.is_dashing {
        speed *= DASH_MULTIPLIER;
    } else if keyboard.pressed(KeyCode::ShiftLeft) || keyboard.pressed(KeyCode::ShiftRight) {
        speed *= SPRINT_MULTIPLIER;
    }

    if move_dir == Vec2::ZERO {
        anim.is_moving = false;
        return;
    } else {
        anim.is_moving = true;
    }

    let delta = move_dir.normalize_or_zero() * speed * dt;
    let mut pos = transform.translation.truncate();
    pos = move_with_walls(pos, delta, hitbox.half, &walls.aabbs);

    transform.translation.x = pos.x;
    transform.translation.y = pos.y;
}

fn aabb_intersects(a_center: Vec2, a_half: Vec2, b_center: Vec2, b_half: Vec2) -> bool {
    let d = a_center - b_center;
    d.x.abs() < (a_half.x + b_half.x) && d.y.abs() < (a_half.y + b_half.y)
}

fn move_with_walls(start: Vec2, delta: Vec2, player_half: Vec2, walls: &[(Vec2, Vec2)]) -> Vec2 {
    if walls.is_empty() || delta == Vec2::ZERO {
        return start + delta;
    }

    let mut pos = start;

    pos.x += delta.x;
    for (c, half) in walls.iter().copied() {
        if aabb_intersects(pos, player_half, c, half) {
            if delta.x > 0.0 {
                pos.x = c.x - half.x - player_half.x;
            } else if delta.x < 0.0 {
                pos.x = c.x + half.x + player_half.x;
            }
        }
    }

    pos.y += delta.y;
    for (c, half) in walls.iter().copied() {
        if aabb_intersects(pos, player_half, c, half) {
            if delta.y > 0.0 {
                pos.y = c.y - half.y - player_half.y;
            } else if delta.y < 0.0 {
                pos.y = c.y + half.y + player_half.y;
            }
        }
    }

    pos
}

fn update_player_animation(
    time: Res<Time>,
    mut query: Query<(&mut Sprite, &mut PlayerAnimation), With<Player>>,
) {
    for (mut sprite, mut anim) in &mut query {
        if !anim.initialized {
            continue;
        }

        anim.timer.tick(time.delta());

        if anim.is_moving {
            if anim.timer.just_finished() {
                anim.frame = (anim.frame + 1) % anim.columns.max(1);
            }
        } else {
            anim.frame = 0;
        }

        update_sprite_rect(&mut sprite, &anim);
    }
}

fn update_sprite_rect(sprite: &mut Sprite, anim: &PlayerAnimation) {
    let frame_w = anim.frame_size.x;
    let frame_h = anim.frame_size.y;
    if frame_w <= 0.0 || frame_h <= 0.0 {
        return;
    }

    let col = anim.frame as f32;
    let row = anim.direction.row_index() as f32;

    let min = Vec2::new(col * frame_w, row * frame_h);
    let max = min + anim.frame_size;

    sprite.rect = Some(Rect { min, max });
}

fn follow_player_camera(
    player_query: Query<&Transform, With<Player>>,
    mut camera_query: Query<&mut Transform, (With<PlayerCamera>, Without<Player>)>,
) {
    let Ok(player_transform) = player_query.single() else {
        return;
    };
    let Ok(mut camera_transform) = camera_query.single_mut() else {
        return;
    };

    camera_transform.translation.x = player_transform.translation.x;
    camera_transform.translation.y = player_transform.translation.y;
}

fn attach_ldtk_player(
    mut commands: Commands,
    query: Query<(Entity, &EntityInstance), Added<EntityInstance>>,
    sprite_q: Query<&Sprite>,
    asset_server: Res<AssetServer>,
) {
    for (entity, instance) in &query {
        if instance.identifier == "Player" {
            let has_sprite = sprite_q.get(entity).is_ok();

            if !has_sprite {
                let texture: Handle<Image> = asset_server.load("player.png");
                let mut sprite = Sprite::from_image(texture);
                sprite.custom_size = Some(Vec2::splat(48.0));
                sprite.color = Color::WHITE;

                commands
                    .entity(entity)
                    .insert((sprite, PlayerAnimation::default()));
            } else {
                commands.entity(entity).insert(PlayerAnimation::default());
            }

            commands.entity(entity).insert((
                Player,
                PlayerDash::default(),
                Health {
                    current: 100.0,
                    max: 100.0,
                },
            ));
        }
    }
}

#[derive(Resource, Default)]
struct PlayerSpawnedFromLdtk(pub bool);

impl Plugin for MovementPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<PlayerSpawnedFromLdtk>()
            .add_systems(OnEnter(GameState::InGame), reset_player_spawn_flag)
            .add_systems(
                Update,
                (
                    spawn_or_move_player_from_ldtk
                        .run_if(in_state(GameState::InGame))
                        .before(apply_player_movement),
                    init_player_animation.run_if(in_state(GameState::InGame)),
                    apply_player_movement.run_if(in_state(GameState::InGame)),
                    update_player_animation.run_if(in_state(GameState::InGame)),
                    follow_player_camera.run_if(in_state(GameState::InGame)),
                ),
            );
    }
}

fn reset_player_spawn_flag(mut flag: ResMut<PlayerSpawnedFromLdtk>) {
    flag.0 = false;
}

use bevy::ecs::hierarchy::ChildOf;

fn spawn_or_move_player_from_ldtk(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut flag: ResMut<PlayerSpawnedFromLdtk>,
    spawn_points: Query<(Entity, &EntityInstance)>,
    parents: Query<&ChildOf>,
    transforms: Query<&Transform, Without<Player>>,
    mut player_q: Query<&mut Transform, With<Player>>,
) {
    if flag.0 {
        return;
    }

    let Some((spawn_e, _inst)) = spawn_points
        .iter()
        .find(|(_, inst)| inst.identifier == "PlayerSpawn" || inst.identifier == "Player")
    else {
        return;
    };

    if parents.get(spawn_e).is_err() {
        return;
    }

    let mut world = Vec3::ZERO;
    let mut cur = Some(spawn_e);
    while let Some(e) = cur {
        if let Ok(t) = transforms.get(e) {
            world += t.translation;
        }
        cur = parents.get(e).ok().map(|p| p.parent());
    }

    world.z = 10.0;

    if let Ok(mut t) = player_q.single_mut() {
        t.translation = world;
    } else {
        let texture: Handle<Image> = asset_server.load("player.png");
        let mut sprite = Sprite::from_image(texture);
        sprite.custom_size = Some(Vec2::splat(24.0));

        commands.spawn((
            sprite,
            Transform::from_translation(world),
            Player,
            PlayerAnimation::default(),
            PlayerDash::default(),
            PlayerHitbox::default(),
            Health {
                current: 100.0,
                max: 100.0,
            },
        ));
    }

    flag.0 = true;
}

pub(crate) fn toggle_debug_colliders(
    keys: Res<ButtonInput<KeyCode>>,
    mut dbg: ResMut<DebugColliders>,
) {
    if keys.just_pressed(KeyCode::F3) {
        dbg.0 = !dbg.0;
        info!("DebugColliders = {}", dbg.0);
    }
}

#[derive(Resource, Default)]
pub(crate) struct DebugColliders(pub bool);

pub(crate) fn draw_colliders_gizmos(
    _dbg: Res<DebugColliders>,
    _walls: Res<crate::ldtk_collision::WallColliders>,
    _mut_gizmos: Gizmos,
    _player: Query<(&Transform, &PlayerHitbox), With<Player>>,
) {
    // 若需要在调试时绘制碰撞箱，可在此实现。当前保留空实现以免未实现时报错。
}
