pub mod types;
pub mod main_menu;
pub mod pause_menu;
pub mod settings;
pub mod save;

use bevy::prelude::*;

use types::GameSettings;
use types::SelectedSlot;

pub use main_menu::MainMenuBackground;

pub struct MenuPlugin;

impl Plugin for MenuPlugin {
    fn build(&self, app: &mut App) {
        // 初始化公共资源
        app.init_resource::<GameSettings>()
            .init_resource::<SelectedSlot>();

        // main menu
        app.add_systems(OnEnter(crate::state::GameState::MainMenu), main_menu::spawn_main_menu)
            .add_systems(OnExit(crate::state::GameState::MainMenu), main_menu::cleanup_main_menu)
            .add_systems(
                Update,
                main_menu::handle_main_menu_buttons.run_if(in_state(crate::state::GameState::MainMenu)),
            );

        // pause menu
        app.add_systems(OnEnter(crate::state::GameState::Paused), pause_menu::spawn_pause_menu)
            .add_systems(OnExit(crate::state::GameState::Paused), pause_menu::cleanup_pause_menu)
            .add_systems(
                Update,
                pause_menu::handle_pause_menu_buttons.run_if(in_state(crate::state::GameState::Paused)),
            );

        // settings
        app.add_systems(
            Update,
            (
                settings::spawn_settings_panel_if_requested,
                settings::handle_settings_buttons,
                settings::sync_settings_texts,
                settings::close_settings_on_esc,
            )
                .chain(),
        );

        

        // save
        app.add_systems(Update, save::handle_save_panel_actions);
        app.add_systems(Update, save::sync_save_slots_list);
        app.add_systems(Update, save::handle_activate_button);
        app.add_systems(Update, save::close_save_panel_on_esc);
        app.add_systems(Update, save::handle_save_slot_buttons);
    }
}