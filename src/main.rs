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
        ))
        .add_systems(Startup, setup_camera)
        .run();
}

fn setup_camera(mut commands: Commands) {
    commands.spawn((Camera2d, PlayerCamera));
}

pub struct LdtkLoaderPlugin {
    pub path: &'static str,
}
impl Plugin for LdtkLoaderPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(LdtkPlugin);
        app.add_systems(Startup, spawn_ldtk_world);
        app.add_systems(Update, handle_level_loaded);
    }
}

fn spawn_ldtk_world(mut commands: Commands, asset_server: Res<AssetServer>) {
    // Spawn camera (if your main.rs 已 spawn camera 则可以省略)
    commands.spawn(Camera2dBundle::default());

    // 加载 ldtk project（路径相对 assets/）
    let ldtk_handle: Handle<LdtkProject> = asset_server.load("ldtk/world.ldtk");

    commands.spawn(LdtkWorldBundle {
        ldtk_handle,
        ..Default::default()
    });
}

fn handle_level_loaded(
    mut events: EventReader<LevelEvent>,
    level_query: Query<Entity, With<Level>>,
    // 你后续可能需要下面这些用于生成碰撞/实体
    mut commands: Commands,
    ldtk_assets: Res<Assets<LdtkProject>>,
) {
    for ev in events.iter() {
        match ev {
            LevelEvent::Loaded { level } => {
                info!("Level loaded: {:?}", level.level);
                // 这里 level.level 是 LdtkLevel 或者 LevelSelection 的引用
                // 你可以查询 spawn 出来的实体（Level root）
                // 例如：
                // let level_entity = level_query.get_single().unwrap();
                // 然后基于该 level_entity 寻找 layer Instances / intgrid / entities
                // 下面我会给更具体的 IntGrid/Entity 处理范例
            }
            _ => {}
        }
    }
}
