use bevy::input::keyboard::KeyCode;
use bevy::prelude::*;

use crate::health::Health;
use crate::input::MovementInput;
use crate::state::GameState;

pub struct MovementPlugin;

#[derive(Component)]
pub struct Player;

#[derive(Component)]
pub struct PlayerCamera;

#[derive(Component)]
pub struct Background;

// 基础行走速度（单位/秒）
const PLAYER_SPEED: f32 = 200.0;
// 疾跑倍率（按住 Shift）
const SPRINT_MULTIPLIER: f32 = 1.5;
// 冲刺速度倍率（由技能触发时使用）
const DASH_MULTIPLIER: f32 = 3.0;
// 冲刺持续时间（秒）
pub const DASH_DURATION: f32 = 0.4;
// 冲刺冷却时间（秒）
pub const DASH_COOLDOWN: f32 = 10.0;

/// 主角朝向
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum PlayerDirection {
    Down,
    Left,
    Right,
    Up,
}

impl PlayerDirection {
    /// 行索引：假定 spritesheet 从上到下依次为：下 / 左 / 右 / 上
    pub fn row_index(self) -> usize {
        match self {
            PlayerDirection::Down => 0,
            PlayerDirection::Left => 1,
            PlayerDirection::Right => 2,
            PlayerDirection::Up => 3,
        }
    }

    /// 转为一个单位方向向量（用于技能朝向兜底）
    pub fn as_vec2(self) -> Vec2 {
        match self {
            PlayerDirection::Down => Vec2::new(0.0, -1.0),
            PlayerDirection::Up => Vec2::new(0.0, 1.0),
            PlayerDirection::Left => Vec2::new(-1.0, 0.0),
            PlayerDirection::Right => Vec2::new(1.0, 0.0),
        }
    }
}

/// 行走图动画数据
#[derive(Component, Debug)]
pub struct PlayerAnimation {
    /// 当前朝向（对外公开，技能系统用它来决定“站立时”的施法方向）
    pub direction: PlayerDirection,
    /// 当前是否在移动（对外公开，可选）
    pub is_moving: bool,

    /// 当前帧列索引
    frame: usize,
    /// 每行的帧数（列数）
    columns: usize,
    /// 行数，固定为 4（上下左右）
    rows: usize,
    /// 是否已根据贴图初始化
    initialized: bool,
    /// 单帧像素宽高
    frame_size: Vec2,
    /// 播放帧的计时器
    timer: Timer,
}

/// 默认：朝下站立
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

/// 冲刺状态组件
///
/// 注意：冲刺的“触发”由技能系统在 skills.rs 里完成，
/// movement.rs 只负责根据这些状态来更新位置和朝向。
#[derive(Component, Default, Debug)]
pub struct PlayerDash {
    pub is_dashing: bool,
    /// 冲刺剩余时间
    pub remaining: f32,
    /// 冲刺冷却剩余时间
    pub cooldown: f32,
    /// 冲刺方向（单位向量）
    pub direction: Vec2,
}

impl Plugin for MovementPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            OnEnter(GameState::InGame),
            (spawn_background_once, spawn_player_once),
        )
        .add_systems(
            Update,
            (
                init_player_animation.run_if(in_state(GameState::InGame)),
                apply_player_movement.run_if(in_state(GameState::InGame)),
                update_player_animation.run_if(in_state(GameState::InGame)),
                follow_player_camera.run_if(in_state(GameState::InGame)),
            ),
        );
    }
}

/// 只在第一次进入 InGame 时生成玩家
fn spawn_player_once(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    player_query: Query<Entity, With<Player>>,
) {
    if player_query.is_empty() {
        let texture: Handle<Image> = asset_server.load("player.png");

        let mut sprite = Sprite::from_image(texture);
        // 显示大小（世界单位），可按需要调整
        sprite.custom_size = Some(Vec2::splat(48.0));
        sprite.color = Color::WHITE;

        commands.spawn((
            sprite,
            Transform::from_xyz(0.0, 0.0, 0.0),
            Player,
            PlayerAnimation::default(),
            PlayerDash::default(),
            // 给玩家一个初始生命值，方便 UI 显示
            Health {
                current: 100.0,
                max: 100.0,
            },
        ));
    }
}

/// 只在第一次进入 InGame 时生成背景
fn spawn_background_once(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    background_query: Query<Entity, With<Background>>,
) {
    if background_query.is_empty() {
        let texture: Handle<Image> = asset_server.load("background.png");

        let mut sprite = Sprite::from_image(texture);
        sprite.custom_size = Some(Vec2::new(1920.0, 1080.0));
        sprite.color = Color::WHITE;


        commands.spawn((
            sprite,
            Transform::from_xyz(0.0, 0.0, -100.0),
            Background,
        ));
    }
}

/// 等待 player.png 资源加载完成后，自动计算帧大小和列数，初始化动画
fn init_player_animation(
    images: Res<Assets<Image>>,
    mut query: Query<(&mut Sprite, &mut PlayerAnimation), With<Player>>,
) {
    for (mut sprite, mut anim) in &mut query {
        if anim.initialized {
            continue;
        }

        // Sprite 里带有纹理句柄
        let Some(image) = images.get(&sprite.image) else {
            continue;
        };

        // 贴图总尺寸（像素）
        let size = image.size(); // UVec2
        let tex_width = size.x as f32;
        let tex_height = size.y as f32;

        // 总共 4 行（下 / 左 / 右 / 上）
        let rows = anim.rows as f32;
        let frame_height = tex_height / rows;
        // 假设单帧为正方形：宽度 = 高度
        let frame_width = frame_height;

        // 列数 = 贴图宽度 / 帧宽（向下取整，至少为 1）
        let columns = (tex_width / frame_width).floor().max(1.0) as usize;

        anim.columns = columns;
        anim.frame_size = Vec2::new(frame_width, frame_height);
        anim.initialized = true;

        // 初始显示：朝下第 0 帧
        update_sprite_rect(&mut sprite, &anim);
    }
}

/// 根据输入移动玩家，并更新动画方向/是否移动
///
/// 这里不负责“启动冲刺”，只根据 PlayerDash 的状态来加速移动。
fn apply_player_movement(
    time: Res<Time>,
    keyboard: Res<ButtonInput<KeyCode>>,
    movement: Res<MovementInput>,
    mut query: Query<(&mut Transform, &mut PlayerAnimation, &mut PlayerDash), With<Player>>,
) {
    let dt = time.delta_secs();

    let Ok((mut transform, mut anim, mut dash)) = query.single_mut() else {
        return;
    };

    let input_dir = movement.0;
    let mut move_dir = input_dir;

    // 更新冲刺冷却
    if dash.cooldown > 0.0 {
        dash.cooldown -= dt;
        if dash.cooldown < 0.0 {
            dash.cooldown = 0.0;
        }
    }

    // 处理冲刺持续时间
    if dash.is_dashing {
        dash.remaining -= dt;
        if dash.remaining <= 0.0 {
            dash.is_dashing = false;
        }
    }

    // 正在冲刺时，移动方向锁定为冲刺方向
    if dash.is_dashing {
        move_dir = dash.direction;
    }

    // 根据方向更新 PlayerAnimation 的朝向
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

    // 计算最终速度
    let mut speed = PLAYER_SPEED;

    // 冲刺：覆盖速度
    if dash.is_dashing {
        speed *= DASH_MULTIPLIER;
    } else {
        // 疾跑：按住 Shift（只有在非冲刺时生效）
        if keyboard.pressed(KeyCode::ShiftLeft) || keyboard.pressed(KeyCode::ShiftRight) {
            speed *= SPRINT_MULTIPLIER;
        }
    }

    // 实际位移
    if move_dir == Vec2::ZERO {
        anim.is_moving = false;
        return;
    } else {
        anim.is_moving = true;
    }

    let delta = move_dir.normalize_or_zero().extend(0.0) * speed * dt;
    transform.translation += delta;
}

/// 播放行走帧，并把帧映射到 sprite.rect 上
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
            // 不动时回到该方向的第 0 帧
            anim.frame = 0;
        }

        update_sprite_rect(&mut sprite, &anim);
    }
}

/// 根据动画数据计算 rect（裁剪 player.png 的某一帧）
fn update_sprite_rect(sprite: &mut Sprite, anim: &PlayerAnimation) {
    let frame_w = anim.frame_size.x;
    let frame_h = anim.frame_size.y;
    if frame_w <= 0.0 || frame_h <= 0.0 {
        return;
    }

    let col = anim.frame as f32;
    let row = anim.direction.row_index() as f32;

    // 纹理坐标：像素空间
    let min = Vec2::new(col * frame_w, row * frame_h);
    let max = min + anim.frame_size;

    sprite.rect = Some(Rect { min, max });
}

/// 相机跟随玩家
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