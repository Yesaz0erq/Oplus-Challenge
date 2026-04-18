use bevy::prelude::*;
use bevy::window::{WindowMode, WindowPlugin, WindowResolution};
use bevy_ecs_ldtk::prelude::*;

mod combat;
mod combat_core;
mod debug_tools;
mod dialogue;
mod enemy;
mod enemy_combat;
mod equipment;
mod exit;
mod game_over_ui;
mod health;
mod i18n;
mod input;
mod interaction;
mod inventory;
mod ldtk_collision;
mod ldtk_gameplay;
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
    debug_tools::DebugToolsPlugin,
    dialogue::DialoguePlugin,
    enemy::EnemyPlugin,
    enemy_combat::EnemyCombatPlugin,
    equipment::EquipmentPlugin,
    exit::ExitPlugin,
    game_over_ui::GameOverUiPlugin,
    health::HealthPlugin,
    input::InputPlugin,
    interaction::InteractionPlugin,
    ldtk_collision::LdtkCollisionPlugin,
    ldtk_gameplay::LdtkGameplayPlugin,
    movement::{Background, MovementPlugin, Player, PlayerCamera},
    save::SavePlugin,
    skills::SkillPlugin,
    skills_pool::SkillPoolPlugin,
    state::GameState,
    ui::MenuPlugin,
};

fn main() {
    App::new()
        .add_plugins(
            DefaultPlugins
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        mode: WindowMode::Windowed,
                        resolution: WindowResolution::from((1280u32, 720u32)),
                        title: "Oplus".into(),
                        ..default()
                    }),
                    ..default()
                })
                .set(ImagePlugin::default_nearest()),
        )
        .add_plugins(LdtkPlugin)
        .init_state::<GameState>()
        .add_plugins(OplusPlugin)
        .run();
}

pub struct OplusPlugin;

impl Plugin for OplusPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            InputPlugin,
            MovementPlugin,
            InteractionPlugin,
            DialoguePlugin,
            ExitPlugin,
            HealthPlugin,
            EquipmentPlugin,
            EnemyPlugin,
            SkillPoolPlugin,
        ));
        app.add_plugins((
            CombatCorePlugin,
            CombatPlugin,
            EnemyCombatPlugin,
            SkillPlugin,
            SavePlugin,
            MenuPlugin,
            DebugToolsPlugin,
            GameOverUiPlugin,
            LdtkCollisionPlugin,
            LdtkGameplayPlugin,
        ));

        app.add_systems(Startup, setup_camera);

        app.add_systems(
            OnEnter(GameState::MainMenu),
            (
                cleanup_world_for_title,
                cleanup_ldtk_world,
                reset_camera_for_main_menu,
            ),
        );

        app.add_systems(OnEnter(GameState::InGame), spawn_ldtk_world_if_missing);

        app.add_systems(
            Update,
            (handle_ldtk_events, on_level_entity_added).run_if(in_state(GameState::InGame)),
        );
    }
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

    commands.insert_resource(LevelSelection::Identifier("Level_0".to_string()));
    commands.insert_resource(LdtkSettings {
        level_spawn_behavior: LevelSpawnBehavior::UseWorldTranslation {
            load_level_neighbors: false,
        },
        ..Default::default()
    });

    commands.spawn(LdtkWorldBundle {
        ldtk_handle: asset_server.load("world.ldtk").into(),
        ..Default::default()
    });
}

fn cleanup_ldtk_world(mut commands: Commands, worlds: Query<Entity, With<LdtkProjectHandle>>) {
    for e in worlds.iter() {
        commands.entity(e).despawn();
    }
}

fn cleanup_world_for_title(
    mut commands: Commands,
    worlds: Query<Entity, With<LdtkProjectHandle>>,
    players: Query<Entity, With<Player>>,
    legacy_bg: Query<Entity, With<Background>>,
) {
    for e in players.iter() {
        commands.entity(e).despawn();
    }
    for e in legacy_bg.iter() {
        commands.entity(e).despawn();
    }
    for e in worlds.iter() {
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
    background_query: Query<Entity, With<Background>>,
) {
    for (entity, level_iid) in query.iter() {
        info!(
            "LDtk Level spawned: entity={:?}, iid={:?}",
            entity, level_iid
        );
        for bg in background_query.iter() {
            commands.entity(bg).despawn();
        }
    }
}

fn reset_camera_for_main_menu(mut q: Query<(&mut Transform, &mut Projection), With<PlayerCamera>>) {
    if let Ok((mut tf, mut projection)) = q.single_mut() {
        tf.translation.x = 0.0;
        tf.translation.y = 0.0;
        if let Projection::Orthographic(ortho) = projection.as_mut() {
            ortho.scale = 1.0;
        }
    }
}
