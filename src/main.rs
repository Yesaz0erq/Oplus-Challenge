use bevy::prelude::*;
use bevy::window::{WindowPlugin, WindowResolution, WindowMode};
use bevy_ecs_ldtk::prelude::*;

mod combat;
mod combat_core;
mod enemy;
mod enemy_combat;
mod equipment;
mod exit;
mod health;
mod input;
mod interaction;
mod inventory;
mod inventory_ui;
mod ldtk_collision;
mod movement;
mod save;
mod skills;
mod skills_pool;
mod state;
mod ui;
mod utils;

use crate::{
    combat::CombatPlugin,
    combat_core::CombatCorePlugin,
    enemy::EnemyPlugin,
    enemy_combat::EnemyCombatPlugin,
    equipment::EquipmentPlugin,
    exit::ExitPlugin,
    health::HealthPlugin,
    input::InputPlugin,
    interaction::InteractionPlugin,
    inventory_ui::InventoryUiPlugin,
    ldtk_collision::LdtkCollisionPlugin,
    movement::MovementPlugin,
    save::SavePlugin,
    skills::SkillPlugin,
    skills_pool::SkillPoolPlugin,
    state::GameState,
    ui::MenuPlugin,
};

fn main() {
    let mut app = App::new();

    app.add_plugins(
        DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                mode: WindowMode::Windowed,
                resolution: WindowResolution::from((1280u32, 720u32)),
                title: "Oplus".into(),
                ..default()
            }),
            ..default()
        })
        .set(ImagePlugin::default_nearest()),
    );

    // LDtk plugin
    app.add_plugins(LdtkPlugin);

    // init game state type
    app.init_state::<GameState>();

    // Add project plugins (single .add_plugins avoids tuple-size trait limit)
    app.add_plugins(InputPlugin);
    app.add_plugins(MovementPlugin);
    app.add_plugins(InteractionPlugin);
    app.add_plugins(ExitPlugin);
    app.add_plugins(HealthPlugin);
    app.add_plugins(EquipmentPlugin);
    app.add_plugins(InventoryUiPlugin);
    app.add_plugins(EnemyPlugin);
    app.add_plugins(SkillPoolPlugin);
    app.add_plugins(CombatCorePlugin);
    app.add_plugins(CombatPlugin);
    app.add_plugins(EnemyCombatPlugin);
    app.add_plugins(SkillPlugin);
    app.add_plugins(SavePlugin);
    app.add_plugins(MenuPlugin);
    app.add_plugins(LdtkCollisionPlugin);

    // Common systems (camera / ldtk handlers)
    app.add_systems(Startup, setup_camera);
    app.add_systems(OnEnter(GameState::MainMenu), cleanup_world_for_title);
    app.add_systems(OnEnter(GameState::InGame), spawn_ldtk_world_if_missing);
    app.add_systems(OnEnter(GameState::MainMenu), cleanup_ldtk_world);
    app.add_systems(Update, handle_ldtk_events.run_if(in_state(GameState::InGame)));
    app.add_systems(Update, on_level_entity_added.run_if(in_state(GameState::InGame)));

    app.run();
}

fn setup_camera(mut commands: Commands) {
    commands.spawn((Camera2d, crate::movement::PlayerCamera));
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
    players: Query<Entity, With<crate::movement::Player>>,
    legacy_bg: Query<Entity, With<crate::movement::Background>>,
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
    }
}