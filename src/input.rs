use bevy::prelude::*;

use crate::state::GameState;

pub struct InputPlugin;

#[derive(Resource, Default)]
pub struct MovementInput(pub Vec2);

impl Plugin for InputPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<MovementInput>()
            // 只在 InGame 时缓存移动方向
            .add_systems(
                Update,
                cache_movement_input.run_if(in_state(GameState::InGame)),
            )
            // 只在主菜单时，按 Enter 进入游戏
            .add_systems(
                Update,
                start_game_from_menu.run_if(in_state(GameState::MainMenu)),
            )
            // InGame 或 Paused 时可以用 ESC 切换暂停
            .add_systems(Update, toggle_pause.run_if(in_game_or_paused));
    }
}

/// 运行条件：当前状态是 InGame 或 Paused
fn in_game_or_paused(state: Res<State<GameState>>) -> bool {
    matches!(state.get(), GameState::InGame | GameState::Paused)
}

/// 读取键盘 WASD，缓存成单位向量
fn cache_movement_input(
    mut movement: ResMut<MovementInput>,
    keyboard: Res<ButtonInput<KeyCode>>,
) {
    let mut direction = Vec2::ZERO;

    if keyboard.pressed(KeyCode::KeyW) {
        direction.y += 1.0;
    }
    if keyboard.pressed(KeyCode::KeyS) {
        direction.y -= 1.0;
    }
    if keyboard.pressed(KeyCode::KeyA) {
        direction.x -= 1.0;
    }
    if keyboard.pressed(KeyCode::KeyD) {
        direction.x += 1.0;
    }

    movement.0 = if direction != Vec2::ZERO {
        direction.normalize()
    } else {
        Vec2::ZERO
    };
}

/// 主菜单按 Enter 进入游戏
fn start_game_from_menu(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut next_state: ResMut<NextState<GameState>>,
) {
    if keyboard.just_pressed(KeyCode::Enter) {
        next_state.set(GameState::InGame);
    }
}

/// InGame <-> Paused 的切换
fn toggle_pause(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut next_state: ResMut<NextState<GameState>>,
    current_state: Res<State<GameState>>,
) {
    // 没按下 ESC 就直接返回
    if !keyboard.just_pressed(KeyCode::Escape) {
        return;
    }

    match current_state.get() {
        GameState::InGame => {
            next_state.set(GameState::Paused);
        }
        GameState::Paused => {
            next_state.set(GameState::InGame);
        }
        // 其他状态（MainMenu / GameOver）这里不做处理
        _ => {}
    }
}
