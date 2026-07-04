use bevy::prelude::*;
use chrono::{DateTime, Datelike, Local as ChronoLocal, Utc};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::time::SystemTime;

use crate::equipment::{EquipmentSet, EquippedItems, ItemDatabase, ItemId, PlayerMemory};
use crate::health::Health;
use crate::inventory::{Inventory, ItemStack};
use crate::movement::Player;
use crate::skills::{CarriedSkills, SkillNumericStats, SkillRuntimeStats};
use crate::skills_pool::SkillId;
use crate::state::GameState;

#[derive(Debug, Clone, Message)]
pub struct ManualSaveEvent {
    pub file_name: Option<String>,
    pub slot_index: Option<u32>,
}

#[derive(Debug, Clone, Message)]
pub struct LoadSlotEvent {
    pub file_name: String,
}

#[derive(Debug, Clone, Message)]
pub struct DeleteSlotEvent {
    pub file_name: String,
}

#[derive(Debug, Clone)]
pub struct SaveSlotMeta {
    pub display_name: String,
    pub file_name: String,
    pub is_auto: bool,
    pub created_at: String,
}

#[derive(Resource, Default, Debug)]
pub struct SaveSlots {
    pub slots: Vec<SaveSlotMeta>,
}

#[derive(Resource, Default, Debug)]
pub struct CurrentSlot {
    pub file_name: Option<String>,
}

#[derive(Resource, Default, Debug)]
pub struct PendingLoad {
    pub file_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SavedInventorySlot {
    pub item_id: u32,
    pub count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SavedSkillStat {
    pub skill_id: u32,
    pub damage: f32,
    pub cooldown: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SaveData {
    pub player_x: f32,
    pub player_y: f32,
    pub hp_current: f32,
    pub hp_max: f32,
    #[serde(default)]
    pub inventory_slot_count: usize,
    #[serde(default)]
    pub inventory_slots: Vec<Option<SavedInventorySlot>>,
    #[serde(default)]
    pub equipped_weapon_id: Option<u32>,
    #[serde(default)]
    pub memory_level: u32,
    #[serde(default)]
    pub memory_skill_capacity: usize,
    #[serde(default)]
    pub carried_skill_slots: Vec<Option<u32>>,
    #[serde(default)]
    pub skill_runtime_stats: Vec<SavedSkillStat>,
}

const AUTOSAVE_INTERVAL_SECS: f32 = 60.0;

pub struct SavePlugin;

impl Plugin for SavePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SaveSlots>()
            .init_resource::<CurrentSlot>()
            .init_resource::<PendingLoad>()
            .add_message::<ManualSaveEvent>()
            .add_message::<LoadSlotEvent>()
            .add_message::<DeleteSlotEvent>()
            .add_systems(OnEnter(GameState::MainMenu), load_save_slots_from_disk)
            .add_systems(Update, handle_load_slot_events)
            .add_systems(Update, handle_delete_slot_events)
            .add_systems(
                Update,
                apply_pending_load
                    .run_if(in_state(GameState::InGame).or_else(in_state(GameState::Paused))),
            )
            .add_systems(
                Update,
                handle_manual_save_events
                    .run_if(in_state(GameState::InGame).or_else(in_state(GameState::Paused))),
            )
            .add_systems(
                Update,
                auto_save_every_minute.run_if(in_state(GameState::InGame)),
            );
    }
}

pub fn generate_slot_display_name(index: u32) -> String {
    let now = ChronoLocal::now();
    let yy = now.year() % 100;
    let mm = now.month();
    let dd = now.day();
    format!("{:02}.{:02}.{:02}.{}", yy, mm, dd, index)
}

pub fn refresh_save_slots_from_disk(slots_res: &mut SaveSlots) {
    let dir = saves_dir();
    let mut slots = Vec::new();

    if let Ok(read_dir) = fs::read_dir(&dir) {
        for entry in read_dir.flatten() {
            let path = entry.path();
            if !path.is_file() {
                continue;
            }

            let file_name = match path.file_name().and_then(|s| s.to_str()) {
                Some(s) => s.to_string(),
                None => continue,
            };
            if !file_name.ends_with(".json") {
                continue;
            }

            let display_name = file_name.trim_end_matches(".json").to_string();
            let is_auto = display_name.starts_with("auto_") || display_name == "autosave";

            let created_at = entry
                .metadata()
                .ok()
                .and_then(|m| m.modified().ok())
                .map(system_time_to_local_string)
                .unwrap_or_default();

            slots.push(SaveSlotMeta {
                display_name,
                file_name,
                is_auto,
                created_at,
            });
        }
    }

    sort_slots(&mut slots);
    slots_res.slots = slots;
}

fn saves_dir() -> PathBuf {
    let mut dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    dir.push("saves");
    let _ = fs::create_dir_all(&dir);
    dir
}

fn slot_file_path(file_name: &str) -> PathBuf {
    let mut path = saves_dir();
    path.push(file_name);
    path
}

fn system_time_to_local_string(t: SystemTime) -> String {
    let dt: DateTime<Utc> = t.into();
    dt.with_timezone(&ChronoLocal)
        .format("%Y-%m-%d %H:%M:%S")
        .to_string()
}

fn sort_slots(slots: &mut [SaveSlotMeta]) {
    slots.sort_by(|a, b| {
        a.created_at
            .cmp(&b.created_at)
            .then(a.display_name.cmp(&b.display_name))
    });
}

fn load_save_slots_from_disk(mut slots_res: ResMut<SaveSlots>) {
    refresh_save_slots_from_disk(&mut slots_res);
}

fn handle_load_slot_events(
    mut ev: MessageReader<LoadSlotEvent>,
    mut pending: ResMut<PendingLoad>,
    mut current: ResMut<CurrentSlot>,
) {
    for e in ev.read() {
        pending.file_name = Some(e.file_name.clone());
        current.file_name = Some(e.file_name.clone());
    }
}

fn handle_delete_slot_events(
    mut ev: MessageReader<DeleteSlotEvent>,
    mut slots: ResMut<SaveSlots>,
    mut current: ResMut<CurrentSlot>,
    mut pending: ResMut<PendingLoad>,
) {
    let mut changed = false;

    for e in ev.read() {
        let path = slot_file_path(&e.file_name);
        match fs::remove_file(&path) {
            Ok(_) => {
                changed = true;
            }
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
                changed = true;
            }
            Err(err) => {
                error!("Failed to delete save {:?}: {}", path, err);
                continue;
            }
        }

        if current.file_name.as_deref() == Some(e.file_name.as_str()) {
            current.file_name = None;
        }
        if pending.file_name.as_deref() == Some(e.file_name.as_str()) {
            pending.file_name = None;
        }
    }

    if changed {
        refresh_save_slots_from_disk(&mut slots);
    }
}

fn apply_pending_load(
    mut pending: ResMut<PendingLoad>,
    mut player_q: Query<
        (
            &mut Transform,
            &mut Health,
            &mut Inventory,
            &mut PlayerMemory,
            &mut EquippedItems,
            &mut EquipmentSet,
        ),
        With<Player>,
    >,
    db: Res<ItemDatabase>,
    mut carried: ResMut<CarriedSkills>,
    mut runtime_stats: ResMut<SkillRuntimeStats>,
) {
    let Some(file_name) = pending.file_name.as_ref().cloned() else {
        return;
    };

    let Ok((mut tf, mut hp, mut inv, mut memory, mut equipped, mut equip_set)) =
        player_q.single_mut()
    else {
        return;
    };

    let path = slot_file_path(&file_name);
    let Ok(bytes) = fs::read(path) else {
        pending.file_name = None;
        return;
    };

    let Ok(data) = serde_json::from_slice::<SaveData>(&bytes) else {
        pending.file_name = None;
        return;
    };

    pending.file_name = None;

    tf.translation.x = data.player_x;
    tf.translation.y = data.player_y;

    hp.max = data.hp_max.max(1.0);
    hp.current = data.hp_current.clamp(0.0, hp.max);

    if data.inventory_slot_count > 0 || !data.inventory_slots.is_empty() {
        let target_len = data.inventory_slot_count.max(data.inventory_slots.len());
        let mut slots = Vec::with_capacity(target_len);
        for slot in data.inventory_slots.into_iter().take(target_len) {
            let mapped = slot.and_then(|saved| {
                ItemId::from_u32(saved.item_id).map(|id| ItemStack {
                    id,
                    count: saved.count.max(1),
                })
            });
            slots.push(mapped);
        }
        while slots.len() < target_len {
            slots.push(None);
        }
        inv.slots = slots;
    }

    if let Some(weapon_id) = data.equipped_weapon_id.and_then(ItemId::from_u32) {
        equipped.weapon = weapon_id;
        if let Some(def) = db.weapon(weapon_id) {
            *equip_set = EquipmentSet::from_weapon(def);
        }
    }

    if data.memory_level > 0 {
        memory.level = data.memory_level;
    }
    if data.memory_skill_capacity > 0 {
        memory.skill_capacity = data.memory_skill_capacity;
    }

    if !data.carried_skill_slots.is_empty() {
        let mut slots = [None; 3];
        for (idx, maybe_id) in data.carried_skill_slots.iter().take(3).enumerate() {
            slots[idx] = maybe_id.and_then(SkillId::from_u32);
        }
        carried.slots = slots;
    }

    runtime_stats.0.clear();
    for saved in data.skill_runtime_stats {
        if let Some(skill_id) = SkillId::from_u32(saved.skill_id) {
            runtime_stats.0.insert(
                skill_id,
                SkillNumericStats::new(saved.damage, saved.cooldown.max(0.0)),
            );
        }
    }
}

fn handle_manual_save_events(
    mut ev_save: MessageReader<ManualSaveEvent>,
    player_q: Query<
        (
            &Transform,
            &Health,
            &Inventory,
            &EquippedItems,
            &PlayerMemory,
        ),
        With<Player>,
    >,
    carried: Res<CarriedSkills>,
    runtime_stats: Res<SkillRuntimeStats>,
    mut slots: ResMut<SaveSlots>,
    mut current: ResMut<CurrentSlot>,
) {
    let Ok((tf, hp, inv, equipped, memory)) = player_q.single() else {
        return;
    };

    for ev in ev_save.read() {
        let (display_name, file_name) = match (&ev.file_name, ev.slot_index) {
            (Some(file_name), _) => (
                file_name.trim_end_matches(".json").to_string(),
                file_name.clone(),
            ),
            (None, Some(index)) => {
                let name = generate_slot_display_name(index);
                (name.clone(), format!("{name}.json"))
            }
            (None, None) => {
                let next = next_daily_index(&slots.slots);
                let name = generate_slot_display_name(next);
                (name.clone(), format!("{name}.json"))
            }
        };

        write_save_to_file(
            &file_name,
            tf,
            hp,
            inv,
            equipped,
            memory,
            &carried,
            &runtime_stats,
        );

        if !slots.slots.iter().any(|s| s.file_name == file_name) {
            slots.slots.push(SaveSlotMeta {
                display_name,
                file_name: file_name.clone(),
                is_auto: false,
                created_at: ChronoLocal::now().format("%Y-%m-%d %H:%M:%S").to_string(),
            });
            sort_slots(&mut slots.slots);
        }

        current.file_name = Some(file_name);
    }
}

fn next_daily_index(existing: &[SaveSlotMeta]) -> u32 {
    let now = ChronoLocal::now();
    let y = (now.year() % 100) as u32;
    let m = now.month();
    let d = now.day();

    let mut max_seq = 0u32;
    for slot in existing {
        let parts: Vec<_> = slot.display_name.split('.').collect();
        if parts.len() != 4 {
            continue;
        }
        let (yy, mm, dd, seq) = (
            parts[0].parse::<u32>().ok(),
            parts[1].parse::<u32>().ok(),
            parts[2].parse::<u32>().ok(),
            parts[3].parse::<u32>().ok(),
        );
        if yy == Some(y)
            && mm == Some(m)
            && dd == Some(d)
            && let Some(s) = seq
        {
            max_seq = max_seq.max(s);
        }
    }
    max_seq + 1
}

fn write_save_to_file(
    file_name: &str,
    tf: &Transform,
    hp: &Health,
    inv: &Inventory,
    equipped: &EquippedItems,
    memory: &PlayerMemory,
    carried: &CarriedSkills,
    runtime_stats: &SkillRuntimeStats,
) {
    let mut skill_runtime_stats = runtime_stats
        .0
        .iter()
        .map(|(skill_id, stats)| SavedSkillStat {
            skill_id: skill_id.to_u32(),
            damage: stats.damage,
            cooldown: stats.cooldown,
        })
        .collect::<Vec<_>>();
    skill_runtime_stats.sort_by_key(|v| v.skill_id);

    let data = SaveData {
        player_x: tf.translation.x,
        player_y: tf.translation.y,
        hp_current: hp.current,
        hp_max: hp.max,
        inventory_slot_count: inv.slots.len(),
        inventory_slots: inv
            .slots
            .iter()
            .map(|slot| {
                slot.map(|stack| SavedInventorySlot {
                    item_id: stack.id.to_u32(),
                    count: stack.count,
                })
            })
            .collect(),
        equipped_weapon_id: Some(equipped.weapon.to_u32()),
        memory_level: memory.level,
        memory_skill_capacity: memory.skill_capacity,
        carried_skill_slots: carried
            .slots
            .iter()
            .map(|slot| slot.map(|id| id.to_u32()))
            .collect(),
        skill_runtime_stats,
    };

    let path = slot_file_path(file_name);
    if let Ok(bytes) = serde_json::to_vec_pretty(&data)
        && let Err(e) = fs::write(&path, bytes)
    {
        error!("Failed to write save to {:?}: {}", path, e);
    }
}

fn auto_save_every_minute(
    time: Res<Time>,
    mut timer: Local<Option<Timer>>,
    player_q: Query<
        (
            &Transform,
            &Health,
            &Inventory,
            &EquippedItems,
            &PlayerMemory,
        ),
        With<Player>,
    >,
    carried: Res<CarriedSkills>,
    runtime_stats: Res<SkillRuntimeStats>,
    mut current: ResMut<CurrentSlot>,
    mut slots: ResMut<SaveSlots>,
) {
    if timer.is_none() {
        *timer = Some(Timer::from_seconds(
            AUTOSAVE_INTERVAL_SECS,
            TimerMode::Repeating,
        ));
    }

    let t = timer.as_mut().unwrap();
    if !t.tick(time.delta()).just_finished() {
        return;
    }

    let Ok((tf, hp, inv, equipped, memory)) = player_q.single() else {
        return;
    };

    let file_name = current
        .file_name
        .clone()
        .unwrap_or_else(|| "autosave.json".to_string());

    write_save_to_file(
        &file_name,
        tf,
        hp,
        inv,
        equipped,
        memory,
        &carried,
        &runtime_stats,
    );

    if !slots.slots.iter().any(|s| s.file_name == file_name) {
        slots.slots.push(SaveSlotMeta {
            display_name: file_name.trim_end_matches(".json").to_string(),
            file_name: file_name.clone(),
            is_auto: true,
            created_at: ChronoLocal::now().format("%Y-%m-%d %H:%M:%S").to_string(),
        });
        sort_slots(&mut slots.slots);
    }

    if current.file_name.is_none() {
        current.file_name = Some(file_name);
    }
}
