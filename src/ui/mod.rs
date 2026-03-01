pub mod main_menu;
pub mod pause_menu;
pub mod save;
pub mod settings;
pub mod skin;
pub mod types;

use bevy::prelude::*;

use types::{GameSettings, SelectedSlot};

use crate::state::GameState;

pub struct MenuPlugin;

#[derive(Component)]
pub struct EscBlockingUi;

impl Plugin for MenuPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<GameSettings>()
            .init_resource::<SelectedSlot>()
            .init_resource::<pause_menu::SuppressPauseMenuOnce>();

        app.add_systems(OnEnter(GameState::MainMenu), main_menu::spawn_main_menu)
            .add_systems(OnExit(GameState::MainMenu), main_menu::cleanup_main_menu)
            .add_systems(
                Update,
                (
                    main_menu::handle_main_menu_buttons,
                    main_menu::animate_main_menu_fade,
                    main_menu::sync_main_menu_background_cover,
                )
                    .run_if(in_state(GameState::MainMenu)),
            );

        app.add_systems(OnEnter(GameState::Paused), pause_menu::spawn_pause_menu)
            .add_systems(OnExit(GameState::Paused), pause_menu::cleanup_pause_menu)
            .add_systems(
                Update,
                pause_menu::handle_pause_menu_buttons.run_if(in_state(GameState::Paused)),
            );

        app.add_systems(
            Update,
            (
                settings::spawn_settings_panel_if_requested,
                settings::handle_settings_buttons,
                settings::sync_settings_texts,
                settings::sync_settings_resolution_row_visibility,
                settings::close_settings_on_esc,
                save::sync_save_slots_list,
                save::handle_save_slot_buttons,
                save::handle_activate_button,
                save::handle_delete_button,
                save::close_save_panel_on_esc,
            )
                .after(crate::input::EscInputSet)
                .chain(),
        );
    }
}
