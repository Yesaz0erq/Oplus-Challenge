use bevy::prelude::*;
use bevy_ecs_ldtk::prelude::*;

use crate::movement::{DebugColliders, draw_colliders_gizmos, toggle_debug_colliders};

#[derive(Resource)]
pub struct WallColliders {
    pub half_size: Vec2,
    pub aabbs: Vec<(Vec2, Vec2)>,
    pub walkables: Vec<Vec2>,
    pub bounds: Option<(Vec2, Vec2)>,
    pub dirty: bool,
}

impl Default for WallColliders {
    fn default() -> Self {
        Self {
            half_size: Vec2::splat(8.0),
            aabbs: Vec::new(),
            walkables: Vec::new(),
            bounds: None,
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

fn rebuild_wall_colliders(
    mut walls: ResMut<WallColliders>,
    layer_meta_q: Query<&LayerMetadata>,
    intgrid_q: Query<(&IntGridCell, &GlobalTransform)>,
    tile_q: Query<
        (
            &GlobalTransform,
            Option<&TileMetadata>,
            Option<&TileEnumTags>,
        ),
        (With<GridCoords>, Without<IntGridCell>),
    >,
) {
    if !walls.dirty {
        return;
    }

    if let Some(meta) = layer_meta_q.iter().next() {
        let grid = (meta.grid_size as f32).max(1.0);
        walls.half_size = Vec2::splat(grid * 0.5);
    }

    walls.aabbs.clear();
    walls.walkables.clear();
    walls.bounds = None;

    let half = walls.half_size;
    let mut bounds_min = Vec2::splat(f32::INFINITY);
    let mut bounds_max = Vec2::splat(f32::NEG_INFINITY);
    let mut has_any_cell = false;

    for (cell, gt) in &intgrid_q {
        let center = gt.translation().truncate();
        has_any_cell = true;
        bounds_min = bounds_min.min(center - half);
        bounds_max = bounds_max.max(center + half);
        if cell.value == 1 {
            walls.aabbs.push((center, half));
        } else {
            walls.walkables.push(center);
        }
    }

    if walls.aabbs.is_empty() && walls.walkables.is_empty() {
        for (gt, meta, tags) in &tile_q {
            let center = gt.translation().truncate();
            has_any_cell = true;
            bounds_min = bounds_min.min(center - half);
            bounds_max = bounds_max.max(center + half);
            if is_tile_solid(meta, tags) {
                walls.aabbs.push((center, half));
            } else {
                walls.walkables.push(center);
            }
        }
    }

    if has_any_cell {
        walls.bounds = Some((bounds_min, bounds_max));
    }

    walls.dirty = false;
}

fn is_tile_solid(meta: Option<&TileMetadata>, tags: Option<&TileEnumTags>) -> bool {
    let solid_keywords = ["wall", "solid", "block", "collider", "obstacle"];

    if let Some(meta) = meta {
        let data = meta.data.to_ascii_lowercase();
        if solid_keywords.iter().any(|k| data.contains(k)) {
            return true;
        }
    }

    if let Some(tags) = tags {
        if tags.tags.iter().any(|tag| {
            let lower = tag.to_ascii_lowercase();
            solid_keywords.iter().any(|k| lower.contains(k))
        }) {
            return true;
        }
    }

    false
}
