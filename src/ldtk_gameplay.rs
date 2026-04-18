use std::collections::HashSet;

use bevy::prelude::*;
use bevy_ecs_ldtk::prelude::{EntityInstance, LdtkFields};

use crate::equipment::ItemId;
use crate::health::Health;
use crate::interaction::InteractEvent;
use crate::inventory::Inventory;
use crate::movement::Player;
use crate::save::SaveSlots;
use crate::state::GameState;
use crate::ui::pause_menu::SuppressPauseMenuOnce;
use crate::ui::save::{self, SavePanelOverlay};
use crate::ui::types::GameSettings;

const INTERACT_RANGE: f32 = 36.0;
const REGION_INTERACT_RANGE: f32 = 18.0;
const SECRET_TRIGGER_MARGIN: f32 = 4.0;

pub struct LdtkGameplayPlugin;

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

impl Plugin for LdtkGameplayPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<TriggeredSecretAreas>()
            .add_systems(OnEnter(GameState::MainMenu), clear_triggered_secret_areas)
            .add_systems(
                Update,
                (handle_ldtk_interactables, trigger_secret_areas)
                    .run_if(in_state(GameState::InGame)),
            );
    }
}

fn clear_triggered_secret_areas(mut triggered: ResMut<TriggeredSecretAreas>) {
    triggered.iids.clear();
}

fn trigger_secret_areas(
    mut triggered: ResMut<TriggeredSecretAreas>,
    player_q: Query<&Transform, With<Player>>,
    entities_q: Query<(&EntityInstance, &GlobalTransform)>,
) {
    let Ok(player_tf) = player_q.single() else {
        return;
    };
    let player_pos = player_tf.translation.truncate();

    for (inst, gt) in &entities_q {
        if inst.identifier != "SecretArea" {
            continue;
        }
        if triggered.iids.contains(&inst.iid) {
            continue;
        }
        let rect = entity_rect(inst, gt);
        if point_in_rect(
            player_pos,
            rect.min - Vec2::splat(SECRET_TRIGGER_MARGIN),
            rect.max + Vec2::splat(SECRET_TRIGGER_MARGIN),
        ) {
            let play_jingle = inst
                .get_bool_field("playSecretJingle")
                .copied()
                .unwrap_or(false);
            triggered.iids.insert(inst.iid.clone());
            info!(
                "SecretArea discovered: iid={} playSecretJingle={}",
                inst.iid, play_jingle
            );
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn handle_ldtk_interactables(
    mut ev_interact: MessageReader<InteractEvent>,
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    settings: Res<GameSettings>,
    mut next_state: ResMut<NextState<GameState>>,
    current_state: Res<State<GameState>>,
    mut suppress_pause_menu_once: ResMut<SuppressPauseMenuOnce>,
    save_panel_q: Query<Entity, With<SavePanelOverlay>>,
    mut player_q: Query<(&mut Transform, &mut Health, &mut Inventory), With<Player>>,
    entities_q: Query<(Entity, &EntityInstance, &GlobalTransform)>,
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

        let Ok((_entity, inst, gt)) = entities_q.get(candidate.entity) else {
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
                    info!("Opened save panel from LDtk GameSaver.");
                }
            }
            InteractKind::Exit => {
                info!("Interacted with LDtk Exit: return to title.");
                next_state.set(GameState::MainMenu);
            }
            InteractKind::Teleport => {
                if let Some(target_pos) = resolve_teleport_destination(inst, &entities_q) {
                    player_tf.translation.x = target_pos.x;
                    player_tf.translation.y = target_pos.y;
                    info!("Teleported player via LDtk Teleport.");
                } else {
                    warn!("Teleport has no resolvable destination: iid={}", inst.iid);
                }
            }
            InteractKind::Ladder => {
                let rect = entity_rect(inst, gt);
                let center_x = (rect.min.x + rect.max.x) * 0.5;
                let top_y = rect.min.y + 12.0;
                let bottom_y = rect.max.y - 12.0;
                let target_y = if (player_pos.y - top_y).abs() < (player_pos.y - bottom_y).abs() {
                    bottom_y
                } else {
                    top_y
                };
                player_tf.translation.x = center_x;
                player_tf.translation.y = target_y;
                info!("Used Ladder (initial implementation): moved to ladder endpoint.");
            }
            InteractKind::Item => {
                if apply_item_pickup(&mut inventory, &mut player_hp, inst, &save_slots) {
                    commands.entity(candidate.entity).try_despawn();
                }
            }
        }
    }
}

fn find_best_candidate(
    player_pos: Vec2,
    entities_q: &Query<(Entity, &EntityInstance, &GlobalTransform)>,
) -> Option<Candidate> {
    let mut best: Option<Candidate> = None;

    for (entity, inst, gt) in entities_q.iter() {
        let kind = match inst.identifier.as_str() {
            "Item" => InteractKind::Item,
            "GameSaver" => InteractKind::GameSaver,
            "Teleport" => InteractKind::Teleport,
            "Ladder" => InteractKind::Ladder,
            "Exit" => InteractKind::Exit,
            _ => continue,
        };

        let rect = entity_rect(inst, gt);
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
    source: &EntityInstance,
    entities_q: &Query<(Entity, &EntityInstance, &GlobalTransform)>,
) -> Option<Vec2> {
    let target_ref = source.get_entity_ref_field("destination").ok()?;
    let target_iid = &target_ref.entity_iid;

    for (_e, inst, gt) in entities_q.iter() {
        if &inst.iid != target_iid {
            continue;
        }
        let rect = entity_rect(inst, gt);
        let mut target = rect.center();
        target.y += 8.0;
        return Some(target);
    }
    None
}

fn apply_item_pickup(
    inventory: &mut Inventory,
    player_hp: &mut Health,
    inst: &EntityInstance,
    _save_slots: &SaveSlots,
) -> bool {
    let item_type = inst
        .get_enum_field("type")
        .ok()
        .map(|s| s.as_str())
        .unwrap_or("Unknown");
    let count = inst
        .get_int_field("count")
        .ok()
        .copied()
        .unwrap_or(1)
        .max(1) as u32;

    match item_type {
        "Bow" => {
            let _ = inventory.try_add(ItemId::HunterBow, count, 99);
            info!("Picked LDtk item Bow x{count}");
            true
        }
        "Spell" => {
            let _ = inventory.try_add(ItemId::MagicWand, count, 99);
            info!("Picked LDtk item Spell x{count}");
            true
        }
        "Fire_blade" | "Vorpal_blade" => {
            let _ = inventory.try_add(ItemId::RustySword, count, 99);
            info!("Picked LDtk item {item_type} x{count} (mapped to RustySword)");
            true
        }
        "Healing_potion" => {
            let heal = 25.0 * count as f32;
            player_hp.current = (player_hp.current + heal).clamp(0.0, player_hp.max);
            info!("Used LDtk item Healing_potion x{count}, healed {}", heal);
            true
        }
        "Meat" => {
            let heal = 12.0 * count as f32;
            player_hp.current = (player_hp.current + heal).clamp(0.0, player_hp.max);
            info!("Used LDtk item Meat x{count}, healed {}", heal);
            true
        }
        other => {
            warn!("Unsupported LDtk item type for initial implementation: {other}");
            false
        }
    }
}

#[derive(Clone, Copy)]
struct Rect2 {
    min: Vec2,
    max: Vec2,
}

impl Rect2 {
    fn center(self) -> Vec2 {
        (self.min + self.max) * 0.5
    }
}

fn entity_rect(inst: &EntityInstance, gt: &GlobalTransform) -> Rect2 {
    let pos = gt.translation().truncate();
    let size = Vec2::new(inst.width as f32, inst.height as f32);
    Rect2 {
        min: pos,
        max: pos + size.max(Vec2::splat(1.0)),
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
