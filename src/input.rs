use crate::state::GameState;
use crate::ui::pause_menu::SuppressPauseMenuOnce;
use crate::ui::EscBlockingUi;
use bevy::prelude::*;

pub struct InputPlugin;

#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone)]
pub struct EscInputSet;

#[derive(Resource, Default)]
pub struct MovementInput(pub Vec2);

impl Plugin for InputPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<MovementInput>()
            .add_systems(
                Update,
                cache_movement_input.run_if(in_state(GameState::InGame)),
            )
            .add_systems(
                Update,
                start_game_from_menu.run_if(in_state(GameState::MainMenu)),
            )
            .add_systems(
                Update,
                toggle_pause.run_if(in_game_or_paused).in_set(EscInputSet),
            );
    }
}

fn in_game_or_paused(state: Res<State<GameState>>) -> bool {
    matches!(state.get(), GameState::InGame | GameState::Paused)
}

fn cache_movement_input(mut movement: ResMut<MovementInput>, keyboard: Res<ButtonInput<KeyCode>>) {
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

fn start_game_from_menu(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut next_state: ResMut<NextState<GameState>>,
) {
    if keyboard.just_pressed(KeyCode::Enter) {
        next_state.set(GameState::InGame);
    }
}

fn toggle_pause(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut next_state: ResMut<NextState<GameState>>,
    current_state: Res<State<GameState>>,
    esc_blocking_ui_q: Query<Entity, With<EscBlockingUi>>,
    mut suppress_pause_menu_once: ResMut<SuppressPauseMenuOnce>,
) {
    if !keyboard.just_pressed(KeyCode::Escape) {
        return;
    }

    let has_blocking_ui = !esc_blocking_ui_q.is_empty();

    match current_state.get() {
        GameState::InGame => {
            if has_blocking_ui {
                suppress_pause_menu_once.0 = true;
            }
            next_state.set(GameState::Paused);
        }
        GameState::Paused => {
            if has_blocking_ui {
                return;
            }
            next_state.set(GameState::InGame);
        }
        _ => {}
    }
}
