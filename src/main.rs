// src/main.rs
use bevy::prelude::*;
use bevy_ecs_ldtk::prelude::*;

use exit::ExitPlugin;
use input::InputPlugin;
use interaction::InteractionPlugin;
use movement::{MovementPlugin, PlayerCamera, Player, Background};
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
use ldtk_collision::LdtkCollisionPlugin;

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
mod ldtk_collision;

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
        commands.entity(e).despawn(); // Bevy 0.17：despawn 会按关系清理子层级
    }
}

fn cleanup_world_for_title(
    mut commands: Commands,
    worlds: Query<Entity, With<LdtkProjectHandle>>,
    players: Query<Entity, With<Player>>,
    legacy_bg: Query<Entity, With<Background>>,
) {
    // 清玩家
    for e in &players {
        commands.entity(e).despawn();
    }
    // 清旧贴图背景（如果还有）
    for e in &legacy_bg {
        commands.entity(e).despawn();
    }
    // 清 LDtk 世界（递归会把 level/layer/instances 一起清掉）
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

        // 移除旧的贴图背景（如果还有）
        for bg in &background_query {
            commands.entity(bg).despawn();
        }

        // 不要再 despawn MainMenuBackground（标题页应由 ui.rs 管）
        // 不要再 insert Transform::IDENTITY（会破坏 UseWorldTranslation 的摆放）
        let _ = entity;
        let _ = level_iid;
    }
}