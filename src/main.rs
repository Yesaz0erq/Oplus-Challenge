use bevy::prelude::*;

use exit::ExitPlugin;
use input::InputPlugin;
use interaction::InteractionPlugin;
use movement::{MovementPlugin, PlayerCamera};
use state::GameState;
use ui::MenuPlugin;

use crate::health::HealthPlugin;
use crate::equipment::EquipmentPlugin;
use crate::combat::CombatPlugin;
use crate::skills::SkillPlugin;
use crate::enemy::EnemyPlugin;
use crate::game_over_ui::GameOverUiPlugin;
use crate::save::SavePlugin;

mod exit;
mod input;
mod interaction;
mod movement;
mod state;
mod ui;
mod health;
mod equipment;
mod combat;
mod skills;
mod enemy;  
mod game_over_ui;
mod save;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .init_state::<GameState>()
        .add_plugins((
            InputPlugin,
            MovementPlugin,
            InteractionPlugin,
            ExitPlugin,
            HealthPlugin,
            EquipmentPlugin, 
            CombatPlugin,
            SkillPlugin,
            EnemyPlugin,
            MenuPlugin,
            GameOverUiPlugin,
            SavePlugin, 
        ))
        .add_systems(Startup, setup_camera)
        .run();
}

fn setup_camera(mut commands: Commands) {
    commands.spawn((Camera2d, PlayerCamera));
}
