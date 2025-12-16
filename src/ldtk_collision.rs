// src/ldtk_collision.rs
use bevy::prelude::*;
use bevy_ecs_ldtk::prelude::*;

/// 缓存：所有墙体的 AABB（中心点、半尺寸）
/// - half_size 默认按 LDtk gridSize=16 => half=8 :contentReference[oaicite:3]{index=3}
#[derive(Resource)]
pub struct WallColliders {
    pub half_size: Vec2,
    pub aabbs: Vec<(Vec2, Vec2)>, // (center, half)
    pub dirty: bool,
}

impl Default for WallColliders {
    fn default() -> Self {
        Self {
            half_size: Vec2::splat(8.0),
            aabbs: Vec::new(),
            dirty: true,
        }
    }
}

pub struct LdtkCollisionPlugin;

impl Plugin for LdtkCollisionPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<WallColliders>()
            // 关卡实体一生成，就标记 dirty（下一帧重建墙体缓存）
            .add_systems(Update, mark_dirty_on_level_spawn)
            // 用 PostUpdate，尽量确保 GlobalTransform 已经可用
            .add_systems(PostUpdate, rebuild_wall_colliders);
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
    intgrid_q: Query<(&IntGridCell, &GlobalTransform)>,
) {
    if !walls.dirty && !walls.aabbs.is_empty() {
        return;
    }

    walls.aabbs.clear();

    let half = walls.half_size; 

    for (cell, gt) in &intgrid_q {
        if cell.value == 1 {
            let center = gt.translation().truncate();
            walls.aabbs.push((center, half));
        }
    }

    if !walls.aabbs.is_empty() {
        walls.dirty = false;
    }
}

