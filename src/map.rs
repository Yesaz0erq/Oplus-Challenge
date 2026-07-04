use std::collections::HashSet;

use bevy::prelude::*;

use crate::equipment::ItemId;
use crate::health::Health;
use crate::interaction::InteractEvent;
use crate::inventory::Inventory;
use crate::movement::{DebugColliders, Player, draw_colliders_gizmos, toggle_debug_colliders};
use crate::save::SaveSlots;
use crate::state::GameState;
use crate::ui::pause_menu::SuppressPauseMenuOnce;
use crate::ui::save::{self, SavePanelOverlay};
use crate::ui::types::GameSettings;

// ── Level data (parsed from assets/levels.json, embedded at build time) ────────

const LEVELS_JSON: &str = include_str!("../assets/levels.json");

#[derive(serde::Deserialize, Clone)]
pub struct LevelsData {
    pub grid_size: f32,
    pub levels: Vec<LevelData>,
}

#[derive(serde::Deserialize, Clone)]
pub struct LevelData {
    pub id: String,
    pub px_wid: f32,
    pub px_hei: f32,
    pub c_wid: usize,
    pub c_hei: usize,
    /// Row-major collision grid. 1 = wall, 2 = floor, 0 = empty.
    pub collision: Vec<i32>,
    pub entities: Vec<EntityData>,
}

#[derive(serde::Deserialize, Clone)]
pub struct EntityData {
    pub id: String,
    /// World-space center (Bevy coords, y-up).
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
    pub fields: serde_json::Value,
}

#[derive(Resource, Clone)]
pub struct Levels(pub LevelsData);

impl Levels {
    pub fn get(&self, id: &str) -> Option<&LevelData> {
        self.0.levels.iter().find(|l| l.id == id)
    }

    pub fn level_px_size(&self, id: &str) -> Option<(f32, f32)> {
        self.get(id).map(|l| (l.px_wid, l.px_hei))
    }
}

/// Identifier of the level that should be loaded. Empty = none.
#[derive(Resource, Default)]
pub struct CurrentLevel(pub String);

/// Identifier of the level currently spawned in the world (drives respawn on change).
#[derive(Resource, Default)]
struct SpawnedLevel(Option<String>);

// ── Components ─────────────────────────────────────────────────────────────────

#[derive(Component)]
struct MapTile;

/// An interactable / anchor placed on the map (replaces LDtk's EntityInstance).
#[derive(Component, Clone)]
pub struct MapEntity {
    pub id: String,
    pub size: Vec2,
    pub fields: serde_json::Value,
}

impl MapEntity {
    pub fn field_str(&self, key: &str) -> Option<&str> {
        self.fields.get(key).and_then(|v| v.as_str())
    }
    pub fn field_i64(&self, key: &str) -> Option<i64> {
        self.fields.get(key).and_then(|v| v.as_i64())
    }
    pub fn field_bool(&self, key: &str) -> Option<bool> {
        self.fields.get(key).and_then(|v| v.as_bool())
    }
}

// ── Wall collider resource ─────────────────────────────────────────────────────

#[derive(Resource)]
pub struct WallColliders {
    pub half_size: Vec2,
    pub aabbs: Vec<(Vec2, Vec2)>,
    pub blocked_cells: Vec<Vec2>,
    pub walkables: Vec<Vec2>,
    pub bounds: Option<(Vec2, Vec2)>,
    pub dirty: bool,
}

impl Default for WallColliders {
    fn default() -> Self {
        Self {
            half_size: Vec2::splat(8.0),
            aabbs: Vec::new(),
            blocked_cells: Vec::new(),
            walkables: Vec::new(),
            bounds: None,
            dirty: true,
        }
    }
}

// ── Plugin ─────────────────────────────────────────────────────────────────────

pub struct MapPlugin;

impl Plugin for MapPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(Levels(
            serde_json::from_str::<LevelsData>(LEVELS_JSON)
                .expect("assets/levels.json must be valid level data"),
        ))
        .init_resource::<CurrentLevel>()
        .init_resource::<SpawnedLevel>()
        .init_resource::<WallColliders>()
        .init_resource::<DebugColliders>()
        .init_resource::<TriggeredSecretAreas>()
        .add_systems(OnEnter(GameState::InGame), init_current_level)
        .add_systems(OnEnter(GameState::MainMenu), reset_map_for_title)
        .add_systems(
            Update,
            (
                spawn_level_if_needed.run_if(in_state(GameState::InGame)),
                toggle_debug_colliders,
                (handle_map_interactables, trigger_secret_areas)
                    .run_if(in_state(GameState::InGame)),
            ),
        )
        .add_systems(PostUpdate, draw_colliders_gizmos);
    }
}

fn init_current_level(mut current: ResMut<CurrentLevel>) {
    if current.0.is_empty() {
        current.0 = "Level_0".to_string();
    }
}

fn reset_map_for_title(
    mut commands: Commands,
    mut current: ResMut<CurrentLevel>,
    mut spawned: ResMut<SpawnedLevel>,
    mut triggered: ResMut<TriggeredSecretAreas>,
    parts: Query<Entity, Or<(With<MapTile>, With<MapEntity>)>>,
) {
    for e in &parts {
        commands.entity(e).despawn();
    }
    current.0.clear();
    spawned.0 = None;
    triggered.iids.clear();
}

// ── Level spawning ───────────────────────────────────────────────────────────

fn spawn_level_if_needed(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    levels: Res<Levels>,
    current: Res<CurrentLevel>,
    mut spawned: ResMut<SpawnedLevel>,
    mut walls: ResMut<WallColliders>,
    old_parts: Query<Entity, Or<(With<MapTile>, With<MapEntity>)>>,
) {
    if current.0.is_empty() {
        return;
    }
    if spawned.0.as_deref() == Some(current.0.as_str()) {
        return;
    }

    let Some(level) = levels.get(&current.0) else {
        warn!("CurrentLevel '{}' not found in level data", current.0);
        return;
    };

    // Clear the previous level.
    for e in &old_parts {
        commands.entity(e).despawn();
    }

    let gs = levels.0.grid_size;
    let half = Vec2::splat(gs * 0.5);

    let map_image: Handle<Image> = asset_server.load(format!("maps/{}.png", level.id));
    commands.spawn((
        MapTile,
        Sprite {
            image: map_image,
            custom_size: Some(Vec2::new(level.px_wid, level.px_hei)),
            ..default()
        },
        Transform::from_xyz(level.px_wid * 0.5, level.px_hei * 0.5, 0.0),
    ));

    // Build colliders from the collision grid. The visual tilemap is prerendered
    // into one sprite per level to keep the runtime entity count low.
    walls.half_size = half;
    walls.aabbs.clear();
    walls.blocked_cells.clear();
    walls.walkables.clear();
    let mut bounds_min = Vec2::splat(f32::INFINITY);
    let mut bounds_max = Vec2::splat(f32::NEG_INFINITY);

    for cy in 0..level.c_hei {
        for cx in 0..level.c_wid {
            let idx = cy * level.c_wid + cx;
            let center = cell_center(cx, cy, gs, level.px_hei);

            bounds_min = bounds_min.min(center - half);
            bounds_max = bounds_max.max(center + half);

            // Collision data.
            match level.collision.get(idx).copied().unwrap_or(0) {
                1 => walls.blocked_cells.push(center),
                2 => walls.walkables.push(center),
                _ => {}
            }
        }
    }

    walls.aabbs = merged_wall_aabbs(level, gs);
    walls.bounds = Some((bounds_min, bounds_max));
    walls.dirty = false;

    // Spawn interactable / anchor entities.
    for ent in &level.entities {
        commands.spawn((
            MapEntity {
                id: ent.id.clone(),
                size: Vec2::new(ent.w, ent.h),
                fields: ent.fields.clone(),
            },
            Transform::from_translation(Vec3::new(ent.x, ent.y, 1.0)),
        ));
    }

    spawned.0 = Some(current.0.clone());
    info!(
        "Spawned level '{}' ({} visual sprites, {} walls, {} entities)",
        level.id,
        1,
        walls.aabbs.len(),
        level.entities.len()
    );
}

fn cell_center(cx: usize, cy: usize, gs: f32, px_hei: f32) -> Vec2 {
    Vec2::new(
        cx as f32 * gs + gs * 0.5,
        px_hei - (cy as f32 * gs + gs * 0.5),
    )
}

fn merged_wall_aabbs(level: &LevelData, gs: f32) -> Vec<(Vec2, Vec2)> {
    let mut visited = vec![false; level.collision.len()];
    let mut merged = Vec::new();

    for cy in 0..level.c_hei {
        for cx in 0..level.c_wid {
            let idx = cy * level.c_wid + cx;
            if visited.get(idx).copied().unwrap_or(true)
                || level.collision.get(idx).copied().unwrap_or(0) != 1
            {
                continue;
            }

            let mut width = 1;
            while cx + width < level.c_wid {
                let next_idx = cy * level.c_wid + cx + width;
                if visited[next_idx] || level.collision.get(next_idx).copied().unwrap_or(0) != 1 {
                    break;
                }
                width += 1;
            }

            let mut height = 1;
            'grow: while cy + height < level.c_hei {
                for dx in 0..width {
                    let next_idx = (cy + height) * level.c_wid + cx + dx;
                    if visited[next_idx] || level.collision.get(next_idx).copied().unwrap_or(0) != 1
                    {
                        break 'grow;
                    }
                }
                height += 1;
            }

            for dy in 0..height {
                for dx in 0..width {
                    visited[(cy + dy) * level.c_wid + cx + dx] = true;
                }
            }

            let min_x = cx as f32 * gs;
            let max_x = (cx + width) as f32 * gs;
            let max_y = level.px_hei - cy as f32 * gs;
            let min_y = level.px_hei - (cy + height) as f32 * gs;
            let center = Vec2::new((min_x + max_x) * 0.5, (min_y + max_y) * 0.5);
            let half = Vec2::new((max_x - min_x) * 0.5, (max_y - min_y) * 0.5);
            merged.push((center, half));
        }
    }

    merged
}

// ── Gameplay / interactable systems ───────────────────────────────────────────

const INTERACT_RANGE: f32 = 36.0;
const REGION_INTERACT_RANGE: f32 = 18.0;
const SECRET_TRIGGER_MARGIN: f32 = 4.0;

#[derive(Resource, Default)]
struct TriggeredSecretAreas {
    iids: HashSet<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum InteractKind {
    Item,
    GameSaver,
    Teleport,
    Ladder,
    Exit,
}

#[derive(Clone, Debug)]
struct Candidate {
    entity: Entity,
    kind: InteractKind,
    distance: f32,
}

fn trigger_secret_areas(
    mut triggered: ResMut<TriggeredSecretAreas>,
    player_q: Query<&Transform, With<Player>>,
    entities_q: Query<(&MapEntity, &GlobalTransform)>,
) {
    let Ok(player_tf) = player_q.single() else {
        return;
    };
    let player_pos = player_tf.translation.truncate();

    for (ent, gt) in &entities_q {
        if ent.id != "SecretArea" {
            continue;
        }
        // Use a stable per-position key so a secret is only triggered once.
        let center = gt.translation().truncate();
        let key = format!("{:.0}:{:.0}", center.x, center.y);
        if triggered.iids.contains(&key) {
            continue;
        }
        let rect = entity_rect(ent, gt);
        if point_in_rect(
            player_pos,
            rect.min - Vec2::splat(SECRET_TRIGGER_MARGIN),
            rect.max + Vec2::splat(SECRET_TRIGGER_MARGIN),
        ) {
            let play_jingle = ent.field_bool("playSecretJingle").unwrap_or(false);
            triggered.iids.insert(key.clone());
            info!(
                "SecretArea discovered: key={} playSecretJingle={}",
                key, play_jingle
            );
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn handle_map_interactables(
    mut ev_interact: MessageReader<InteractEvent>,
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    settings: Res<GameSettings>,
    mut next_state: ResMut<NextState<GameState>>,
    current_state: Res<State<GameState>>,
    mut suppress_pause_menu_once: ResMut<SuppressPauseMenuOnce>,
    save_panel_q: Query<Entity, With<SavePanelOverlay>>,
    mut player_q: Query<(&mut Transform, &mut Health, &mut Inventory), With<Player>>,
    entities_q: Query<(Entity, &MapEntity, &GlobalTransform)>,
    save_slots: Res<SaveSlots>,
) {
    if ev_interact.is_empty() {
        return;
    }

    for _ in ev_interact.read() {
        let Ok((mut player_tf, mut player_hp, mut inventory)) = player_q.single_mut() else {
            return;
        };
        let player_pos = player_tf.translation.truncate();

        let Some(candidate) = find_best_candidate(player_pos, &entities_q) else {
            continue;
        };

        let Ok((_entity, ent, gt)) = entities_q.get(candidate.entity) else {
            continue;
        };

        match candidate.kind {
            InteractKind::GameSaver => {
                if save_panel_q.is_empty() {
                    save::open_save_panel(&mut commands, &asset_server, settings.language);
                    if matches!(current_state.get(), GameState::InGame) {
                        suppress_pause_menu_once.0 = true;
                        next_state.set(GameState::Paused);
                    }
                    info!("Opened save panel from GameSaver.");
                }
            }
            InteractKind::Exit => {
                info!("Interacted with Exit: return to title.");
                next_state.set(GameState::MainMenu);
            }
            InteractKind::Teleport => {
                if let Some(target_pos) = resolve_teleport_destination(ent, &entities_q) {
                    player_tf.translation.x = target_pos.x;
                    player_tf.translation.y = target_pos.y;
                    info!("Teleported player.");
                } else {
                    warn!("Teleport has no resolvable destination.");
                }
            }
            InteractKind::Ladder => {
                let rect = entity_rect(ent, gt);
                let center_x = (rect.min.x + rect.max.x) * 0.5;
                let top_y = rect.max.y + 12.0;
                let bottom_y = rect.min.y - 12.0;
                let target_y = if (player_pos.y - top_y).abs() < (player_pos.y - bottom_y).abs() {
                    bottom_y
                } else {
                    top_y
                };
                player_tf.translation.x = center_x;
                player_tf.translation.y = target_y;
                info!("Used Ladder: moved to ladder endpoint.");
            }
            InteractKind::Item => {
                if apply_item_pickup(&mut inventory, &mut player_hp, ent, &save_slots) {
                    commands.entity(candidate.entity).try_despawn();
                }
            }
        }
    }
}

fn find_best_candidate(
    player_pos: Vec2,
    entities_q: &Query<(Entity, &MapEntity, &GlobalTransform)>,
) -> Option<Candidate> {
    let mut best: Option<Candidate> = None;

    for (entity, ent, gt) in entities_q.iter() {
        let kind = match ent.id.as_str() {
            "Item" => InteractKind::Item,
            "GameSaver" => InteractKind::GameSaver,
            "Teleport" => InteractKind::Teleport,
            "Ladder" => InteractKind::Ladder,
            "Exit" => InteractKind::Exit,
            _ => continue,
        };

        let rect = entity_rect(ent, gt);
        let distance = distance_to_rect(player_pos, rect.min, rect.max);
        let max_range = if matches!(kind, InteractKind::Ladder) {
            REGION_INTERACT_RANGE
        } else {
            INTERACT_RANGE
        };
        if distance > max_range {
            continue;
        }

        let candidate = Candidate {
            entity,
            kind,
            distance,
        };

        if is_better_candidate(&candidate, best.as_ref()) {
            best = Some(candidate);
        }
    }

    best
}

fn is_better_candidate(a: &Candidate, b: Option<&Candidate>) -> bool {
    let Some(b) = b else {
        return true;
    };

    let pa = interact_priority(a.kind);
    let pb = interact_priority(b.kind);
    if pa != pb {
        return pa < pb;
    }
    a.distance < b.distance
}

fn interact_priority(kind: InteractKind) -> u8 {
    match kind {
        InteractKind::Item => 0,
        InteractKind::GameSaver => 1,
        InteractKind::Teleport => 2,
        InteractKind::Ladder => 3,
        InteractKind::Exit => 4,
    }
}

fn resolve_teleport_destination(
    source: &MapEntity,
    entities_q: &Query<(Entity, &MapEntity, &GlobalTransform)>,
) -> Option<Vec2> {
    let target_iid = source.field_str("destination")?;

    for (_e, ent, gt) in entities_q.iter() {
        if ent.field_str("iid") != Some(target_iid) {
            continue;
        }
        let mut target = gt.translation().truncate();
        target.y += 8.0;
        return Some(target);
    }
    None
}

fn apply_item_pickup(
    inventory: &mut Inventory,
    player_hp: &mut Health,
    ent: &MapEntity,
    _save_slots: &SaveSlots,
) -> bool {
    let item_type = ent.field_str("type").unwrap_or("Unknown");
    let count = ent.field_i64("count").unwrap_or(1).max(1) as u32;

    match item_type {
        "Bow" => {
            let _ = inventory.try_add(ItemId::HunterBow, count, 99);
            info!("Picked item Bow x{count}");
            true
        }
        "Spell" => {
            let _ = inventory.try_add(ItemId::MagicWand, count, 99);
            info!("Picked item Spell x{count}");
            true
        }
        "Fire_blade" | "Vorpal_blade" => {
            let _ = inventory.try_add(ItemId::RustySword, count, 99);
            info!("Picked item {item_type} x{count} (mapped to RustySword)");
            true
        }
        "Healing_potion" => {
            let heal = 25.0 * count as f32;
            player_hp.current = (player_hp.current + heal).clamp(0.0, player_hp.max);
            info!("Used item Healing_potion x{count}, healed {}", heal);
            true
        }
        "Meat" => {
            let heal = 12.0 * count as f32;
            player_hp.current = (player_hp.current + heal).clamp(0.0, player_hp.max);
            info!("Used item Meat x{count}, healed {}", heal);
            true
        }
        other => {
            warn!("Unsupported item type: {other}");
            false
        }
    }
}

#[derive(Clone, Copy)]
struct Rect2 {
    min: Vec2,
    max: Vec2,
}

fn entity_rect(ent: &MapEntity, gt: &GlobalTransform) -> Rect2 {
    let center = gt.translation().truncate();
    let half = (ent.size * 0.5).max(Vec2::splat(0.5));
    Rect2 {
        min: center - half,
        max: center + half,
    }
}

fn distance_to_rect(point: Vec2, min: Vec2, max: Vec2) -> f32 {
    let dx = if point.x < min.x {
        min.x - point.x
    } else if point.x > max.x {
        point.x - max.x
    } else {
        0.0
    };
    let dy = if point.y < min.y {
        min.y - point.y
    } else if point.y > max.y {
        point.y - max.y
    } else {
        0.0
    };
    Vec2::new(dx, dy).length()
}

fn point_in_rect(point: Vec2, min: Vec2, max: Vec2) -> bool {
    point.x >= min.x && point.y >= min.y && point.x <= max.x && point.y <= max.y
}
