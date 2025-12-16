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

use bevy::prelude::*;
use bevy_ecs_ldtk::prelude::*;

fn spawn_ldtk_world(mut commands: Commands, asset_server: Res<AssetServer>) {
    // 选第 0 关（你也可以换成 Indices / Iid）
    commands.insert_resource(LevelSelection::index(0)); // 

    // 更接近教程/示例的行为：按 LDtk 的 worldX/worldY 来摆放关卡，并可选择加载邻居关
    commands.insert_resource(LdtkSettings {
        level_spawn_behavior: LevelSpawnBehavior::UseWorldTranslation {
            load_level_neighbors: true,
        },
        ..Default::default()
    }); // 

    commands.spawn(LdtkWorldBundle {
        // 注意这里用 into()（bevy_ecs_ldtk 在新版本用 LdtkProjectHandle 作为组件）
        ldtk_handle: asset_server.load("world.ldtk").into(),
        ..Default::default()
    });
}

fn handle_ldtk_events(mut events: MessageReader<LevelEvent>) {
    for ev in events.read() {
        info!("LDtk LevelEvent: {:?}", ev);
    }
}

fn on_level_entity_added(
    mut commands: Commands,
    // 当 LevelIid 第一次被添加到实体上，说明某个 level 实体 spawn 完毕
    query: Query<(Entity, &LevelIid), Added<LevelIid>>,
    // 查找 movement.rs 中使用的 Background tag
    background_query: Query<Entity, With<crate::movement::Background>>,
    // 查找主菜单背景（如果你想在进入 InGame 时也清除）
    main_menu_bg_query: Query<Entity, With<crate::ui::MainMenuBackground>>,
) {
    for (entity, level_iid) in &query {
        info!("LDtk Level spawned: entity={:?}, iid={:?}", entity, level_iid);

        // 1) 移除 movement spawn 的默认背景（如果存在）
        for bg in &background_query {
            info!("Despawning legacy Background entity {:?}", bg);
            commands.entity(bg).despawn();
        }

        // 2) 也移除主菜单背景（进入游戏或加载 level 时通常希望看到地图）
        for m in &main_menu_bg_query {
            info!("Despawning MainMenuBackground entity {:?}", m);
            commands.entity(m).despawn();
        }

        // 3) （可选）若需要，将 level 根实体的 Transform / z 调整到合适 z 层
        //    这里演示把 level 实体提升到 z = 0（确保低于玩家 z）
        //    但很多情况下 LDtk 自身会给 tiles 设置世界坐标，可按需调整。
        commands.entity(entity).insert(Transform::IDENTITY);
    }
}