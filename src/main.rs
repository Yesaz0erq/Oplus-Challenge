use bevy::app::AppExit;
use bevy::log::LogPlugin;
use bevy::prelude::*;
use bevy::window::{WindowMode, WindowPlugin, WindowResolution};

mod combat;
mod debug_tools;
mod dialogue;
mod enemy;
mod equipment;
mod game_over_ui;
mod health;
mod i18n;
mod input;
mod interaction;
mod inventory;
mod map;
mod movement;
mod save;
mod skills;
mod skills_pool;
mod state;
mod ui;
mod utils;

use crate::{
    combat::CombatPlugin,
    debug_tools::DebugToolsPlugin,
    dialogue::DialoguePlugin,
    enemy::EnemyPlugin,
    equipment::EquipmentPlugin,
    game_over_ui::GameOverUiPlugin,
    health::HealthPlugin,
    input::InputPlugin,
    interaction::InteractionPlugin,
    map::MapPlugin,
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
                .set(ImagePlugin::default_nearest())
                .set(LogPlugin {
                    filter: "wgpu=error,naga=warn".into(),
                    ..default()
                }),
        )
        .add_message::<AppExit>()
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
            HealthPlugin,
            EquipmentPlugin,
            EnemyPlugin,
            SkillPoolPlugin,
        ));
        app.add_plugins((
            CombatPlugin,
            SkillPlugin,
            SavePlugin,
            MenuPlugin,
            DebugToolsPlugin,
            GameOverUiPlugin,
            MapPlugin,
        ));

        app.add_systems(Startup, setup_camera);

        app.add_systems(
            OnEnter(GameState::MainMenu),
            (cleanup_world_for_title, reset_camera_for_main_menu),
        );
    }
}

fn setup_camera(mut commands: Commands) {
    commands.spawn((Camera2d, PlayerCamera));
}

fn cleanup_world_for_title(
    mut commands: Commands,
    players: Query<Entity, With<Player>>,
    legacy_bg: Query<Entity, With<Background>>,
) {
    for e in players.iter() {
        commands.entity(e).despawn();
    }
    for e in legacy_bg.iter() {
        commands.entity(e).despawn();
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
