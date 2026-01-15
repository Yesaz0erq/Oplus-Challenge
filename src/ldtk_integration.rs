use bevy::prelude::*;
use bevy_ecs_ldtk::prelude::*;

pub struct LdtkLoaderPlugin {
    pub path: &'static str,
}

impl Plugin for LdtkLoaderPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(LdtkPlugin);
        app.add_systems(Startup, spawn_world(self.path));
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
    intgrid_query: Query<(&IntGridCell, &Transform)>,
    entity_query: Query<(&LdtkEntityInstance, &Transform)>,
    mut commands: Commands,
) {
    for ev in events.iter() {
        if let LevelEvent::Loaded { level } = ev {
            info!("LDtk level loaded: {:?}", level.level.iid);

            for (cell, tf) in &intgrid_query {
                match cell.value {
                    1 => {
                        commands.spawn((Transform::from_translation(tf.translation), Wall));
                    }
                    2 => {
                        commands.spawn((Transform::from_translation(tf.translation), Water));
                    }
                    _ => {}
                }
            }

            for (inst, tf) in &entity_query {
                match inst.identifier.as_str() {
                    "PlayerSpawn" => {
                        commands.spawn((Player, Transform::from_translation(tf.translation)));
                    }
                    "EnemySpawn" => {
                        commands.spawn((
                            Enemy,
                            Transform::from_translation(tf.translation),
                            Health {
                                current: 50.0,
                                max: 50.0,
                            },
                        ));
                    }
                    _ => {}
                }
            }
        }
    }
}
