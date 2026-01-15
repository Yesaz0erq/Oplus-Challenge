use bevy::prelude::*;
use bevy_ecs_ldtk::prelude::*;

use crate::movement::{draw_colliders_gizmos, toggle_debug_colliders, DebugColliders};

#[derive(Resource)]
pub struct WallColliders {
    pub half_size: Vec2,
    pub aabbs: Vec<(Vec2, Vec2)>,
    pub walkables: Vec<Vec2>,
    pub dirty: bool,
}

impl Default for WallColliders {
    fn default() -> Self {
        Self {
            half_size: Vec2::splat(8.0),
            aabbs: Vec::new(),
            walkables: Vec::new(),
            dirty: true,
        }
    }
}

pub struct LdtkCollisionPlugin;

impl Plugin for LdtkCollisionPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<WallColliders>()
            .init_resource::<DebugColliders>()
            .add_systems(Update, toggle_debug_colliders)
            .add_systems(Update, mark_dirty_on_level_spawn)
            .add_systems(
                PostUpdate,
                (
                    rebuild_wall_colliders.after(TransformSystems::Propagate),
                    draw_colliders_gizmos.after(rebuild_wall_colliders),
                ),
            );
    }
}

fn mark_dirty_on_level_spawn(
    mut walls: ResMut<WallColliders>,
    spawned_levels: Query<Entity, Added<LevelIid>>,
) {
    if !spawned_levels.is_empty() {
        walls.dirty = true;
    }
}

fn rebuild_wall_colliders(mut walls: ResMut<WallColliders>, intgrid_q: Query<(&IntGridCell, &GlobalTransform)>) {
    if !walls.dirty && !walls.aabbs.is_empty() && !walls.walkables.is_empty() {
        return;
    }

    walls.aabbs.clear();
    walls.walkables.clear();

    let half = walls.half_size;

    for (cell, gt) in &intgrid_q {
        let center = gt.translation().truncate();
        if cell.value == 1 {
            walls.aabbs.push((center, half));
        } else {
            walls.walkables.push(center);
        }
    }

    if !walls.aabbs.is_empty() || !walls.walkables.is_empty() {
        walls.dirty = false;
    }
}
