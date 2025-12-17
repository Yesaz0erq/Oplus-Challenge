// src/ldtk_integration.rs
use bevy::prelude::*;
use bevy_ecs_ldtk::prelude::*;

pub struct LdtkLoaderPlugin {
    pub path: &'static str,
}

impl Plugin for LdtkLoaderPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(LdtkPlugin);
        app.add_systems(Startup, spawn_world(self.path));
        // 在 Update 阶段处理刚生成的 level，做碰撞/实体生成
        app.add_systems(Update, on_level_spawned);
    }
}

fn spawn_world(path: &'static str) -> impl FnMut(Commands, Res<AssetServer>) + Copy {
    move |mut commands: Commands, asset_server: Res<AssetServer>| {
        commands.spawn(Camera2d::default());
        let handle: Handle<LdtkProject> = asset_server.load(path);
        commands.spawn(LdtkWorldBundle {
            ldtk_handle: handle,
            ..Default::default()
        });
    }
}

fn on_level_spawned(
    mut events: EventReader<LevelEvent>,
    // IntGrid cell / entity instance queries will depend on crate's exact types;
    // below names are placeholders - adapt if compiler complains.
    intgrid_query: Query<(&IntGridCell, &Transform)>,
    entity_query: Query<(&LdtkEntityInstance, &Transform)>,
    mut commands: Commands,
) {
    for ev in events.iter() {
        if let LevelEvent::Loaded { level } = ev {
            info!("LDtk level loaded: {:?}", level.level.iid);

            // 1) handle intgrid → collision
            for (cell, tf) in &intgrid_query {
                match cell.value {
                    1 => {
                        // wall/solid
                        commands.spawn((
                            Transform::from_translation(tf.translation),
                            // Tag, or add Rapier collider etc.
                            Wall,
                        ));
                    }
                    2 => {
                        // water
                        commands.spawn((
                            Transform::from_translation(tf.translation),
                            Water,
                        ));
                    }
                    _ => {}
                }
            }

            // 2) handle entity instances -> spawn game entities
            for (inst, tf) in &entity_query {
                match inst.identifier.as_str() {
                    "PlayerSpawn" => {
                        commands.spawn((
                            Player,
                            Transform::from_translation(tf.translation),
                            // insert bundle, sprite, etc.
                        ));
                    }
                    "EnemySpawn" => {
                        // you may want to read inst.field_instances to get enemy type
                        commands.spawn((
                            Enemy,
                            Transform::from_translation(tf.translation),
                            Health { current: 50.0, max: 50.0 },
                        ));
                    }
                    _ => {}
                }
            }
        }
    }
}
