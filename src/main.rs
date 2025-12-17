// src/main.rs
use bevy::prelude::*;
use bevy_ecs_ldtk::prelude::*;

use crate::{
    combat::CombatPlugin,
    enemy::EnemyPlugin,
    equipment::EquipmentPlugin,
    game_over_ui::GameOverUiPlugin,
    health::HealthPlugin,
    inventory_ui::InventoryUiPlugin,
    save::SavePlugin,
    skills::SkillPlugin,
};

use exit::ExitPlugin;
use input::InputPlugin;
use interaction::InteractionPlugin;
use ldtk_collision::LdtkCollisionPlugin;
use movement::{Background, MovementPlugin, Player, PlayerCamera};
use state::GameState;
use ui::MenuPlugin;

mod combat;
mod enemy;
mod equipment;
mod exit;
mod game_over_ui;
mod health;
mod input;
mod interaction;
mod inventory;
mod inventory_ui;
mod ldtk_collision;
mod movement;
mod save;
mod skills;
mod state;
mod ui;

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
            LdtkCollisionPlugin,
        ))
        .add_systems(Startup, setup_camera)
        .add_systems(OnEnter(GameState::MainMenu), cleanup_world_for_title)
        .add_systems(OnEnter(GameState::InGame), spawn_ldtk_world_if_missing)
        .add_systems(OnEnter(GameState::MainMenu), cleanup_ldtk_world)
        .add_systems(Update, handle_ldtk_events.run_if(in_state(GameState::InGame)))
        .add_systems(Update, on_level_entity_added.run_if(in_state(GameState::InGame)))
        .run();
}

fn setup_camera(mut commands: Commands) {
    commands.spawn((Camera2d, PlayerCamera));
}

fn spawn_ldtk_world_if_missing(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    worlds: Query<Entity, With<LdtkProjectHandle>>,
) {
    if !worlds.is_empty() {
        return;
    }

    commands.insert_resource(LevelSelection::index(0));

    commands.insert_resource(LdtkSettings {
        level_spawn_behavior: LevelSpawnBehavior::UseWorldTranslation {
            load_level_neighbors: true,
        },
        ..Default::default()
    });

    commands.spawn(LdtkWorldBundle {
        ldtk_handle: asset_server.load("world.ldtk").into(),
        ..Default::default()
    });
}

fn cleanup_ldtk_world(mut commands: Commands, worlds: Query<Entity, With<LdtkProjectHandle>>) {
    for e in &worlds {
        commands.entity(e).despawn();
    }
}

fn cleanup_world_for_title(
    mut commands: Commands,
    worlds: Query<Entity, With<LdtkProjectHandle>>,
    players: Query<Entity, With<Player>>,
    legacy_bg: Query<Entity, With<Background>>,
) {
    for e in &players {
        commands.entity(e).despawn();
    }
    for e in &legacy_bg {
        commands.entity(e).despawn();
    }
    for e in &worlds {
        commands.entity(e).despawn();
    }
}

fn handle_ldtk_events(mut events: MessageReader<LevelEvent>) {
    for ev in events.read() {
        info!("LDtk LevelEvent: {:?}", ev);
    }
}

fn on_level_entity_added(
    mut commands: Commands,
    query: Query<(Entity, &LevelIid), Added<LevelIid>>,
    background_query: Query<Entity, With<crate::movement::Background>>,
) {
    for (entity, level_iid) in &query {
        info!("LDtk Level spawned: entity={:?}, iid={:?}", entity, level_iid);

        for bg in &background_query {
            commands.entity(bg).despawn();
        }

        // 升级/初始化逻辑在 movement.rs 等模块负责
        let _ = (entity, level_iid);
    }
}
