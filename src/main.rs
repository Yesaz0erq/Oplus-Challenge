// src/main.rs
use bevy::prelude::*;
use bevy_ecs_ldtk::prelude::*;

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
use crate::inventory_ui::InventoryUiPlugin;

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
mod inventory;
mod inventory_ui;

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
            InventoryUiPlugin,
            MenuPlugin,
            GameOverUiPlugin,
            SavePlugin,
            LdtkPlugin,
        ))
        .add_systems(Startup, setup_camera)
        .add_systems(Startup, spawn_ldtk_world)
        .add_systems(Update, handle_ldtk_events)
        .add_systems(Update, on_level_entity_added)
        .run();
}

fn setup_camera(mut commands: Commands) {
    commands.spawn((Camera2d, PlayerCamera)); 
}

fn spawn_ldtk_world(mut commands: Commands, asset_server: Res<AssetServer>) {
    let ldtk_handle = asset_server.load("world.ldtk").into();

    commands.spawn(LdtkWorldBundle {
        ldtk_handle,
        ..default()
    });

    info!("Spawned LdtkWorldBundle for world.ldtk");
}

fn handle_ldtk_events(mut events: MessageReader<LevelEvent>) {
    for ev in events.read() {
        info!("LDtk LevelEvent: {:?}", ev);
    }
}

fn on_level_entity_added(
    query: Query<(Entity, &LevelIid), Added<LevelIid>>,
) {
    for (entity, level_iid) in &query {
        info!("Level entity spawned: entity={:?}, iid={:?}", entity, level_iid);
    }
}
