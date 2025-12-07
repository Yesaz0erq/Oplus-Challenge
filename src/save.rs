use bevy::prelude::*;
use chrono::{Datelike, Local}; // 提供 year()/month()/day()
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

use crate::health::Health;
use crate::movement::Player;
use crate::state::GameState;

/// 手动保存事件：Some(file) 覆盖该存档；None 创建新存档
#[derive(Event, Debug, Clone)]
pub struct ManualSaveEvent {
    pub file_name: Option<String>,
}

/// 选择加载某一个存档槽位
#[derive(Event, Debug, Clone)]
pub struct LoadSlotEvent {
    /// 要加载的存档文件名，例如 "25.12.06.1.json"
    pub file_name: String,
}

/// 单个存档槽的元数据（用于 UI 列表）
#[derive(Debug, Clone)]
pub struct SaveSlotMeta {
    pub is_auto: bool,
    pub created_at: String,
    /// 显示在 UI 上的名字，例如 "25.12.06.1"
    pub display_name: String,
    /// 实际文件名，例如 "25.12.06.1.json"
    pub file_name: String,
}

/// 所有存档槽列表（从磁盘扫描出来）
#[derive(Resource, Default, Debug)]
pub struct SaveSlots {
    pub slots: Vec<SaveSlotMeta>,
}

/// 当前使用中的存档（自动保存 & 手动保存共用）
#[derive(Resource, Default, Debug)]
pub struct CurrentSlot {
    /// 文件名，例如 "25.12.06.1.json"
    pub file_name: Option<String>,
}

/// 进入游戏时待加载的存档（由菜单/事件设置）
#[derive(Resource, Default, Debug)]
pub struct PendingLoad {
    /// 要加载的文件名；用完后会被置 None
    pub file_name: Option<String>,
}

/// 存档内容（真正写进 json 的结构）
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SaveData {
    #[serde(default)]
    pub is_auto: bool,
    #[serde(default)]
    pub display_name: String,
    #[serde(default)]
    pub created_at: String,
    pub player_x: f32,
    pub player_y: f32,
    pub player_hp_current: f32,
    pub player_hp_max: f32,
}

/// 存档系统插件
pub struct SavePlugin;

impl Plugin for SavePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SaveSlots>()
            .init_resource::<CurrentSlot>()
            .init_resource::<PendingLoad>()
            // 注册“事件类型”（其实是 Message）
            .add_event::<ManualSaveEvent>()
            .add_event::<LoadSlotEvent>()
            // 回到主菜单时，重新扫描硬盘上的所有存档
            .add_systems(OnEnter(GameState::MainMenu), load_save_slots_from_disk)
            // 进入游戏时：决定要用哪个存档，并尝试加载
            .add_systems(
                OnEnter(GameState::InGame),
                (choose_autosave_or_new_slot, apply_pending_load),
            )
            // InGame 中：自动保存 + 处理手动保存 / 读档事件
            .add_systems(
                Update,
                (
                    auto_save_every_n_seconds,
                    handle_manual_save_events,
                    handle_load_slot_events,
                )
                    .run_if(in_state(GameState::InGame)),
            );

        // 暂停菜单也允许手动存档
        app.add_systems(
            Update,
            handle_manual_save_events.run_if(in_state(GameState::Paused)),
        );
    }
}

/// 存档目录：./saves
fn saves_dir() -> PathBuf {
    let mut dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    dir.push("saves");
    let _ = fs::create_dir_all(&dir);
    dir
}

/// 把文件名拼成完整路径
fn slot_file_path(file_name: &str) -> PathBuf {
    let mut path = saves_dir();
    path.push(file_name);
    path
}

// ---------- make this function public so UI can call it ----------
pub fn generate_slot_display_name(index: u32) -> String {
    let now = chrono::Local::now();
    let yy = now.year() % 100;
    let mm = now.month();
    let dd = now.day();
    format!("{:02}.{:02}.{:02}.{}", yy, mm, dd, index)
}

/// 从磁盘扫描所有存档，填充 SaveSlots，用于主菜单 / ESC 菜单 UI 列表
fn load_save_slots_from_disk(mut slots_res: ResMut<SaveSlots>) {
    refresh_save_slots_from_disk(&mut slots_res);
}

// ---------- public helper: refresh save slots from disk ----------
/// Scan ./saves and fill SaveSlots (public for UI to refresh)
pub fn refresh_save_slots_from_disk(slots_res: &mut SaveSlots) {
    let mut dir = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
    dir.push("saves");
    let _ = std::fs::create_dir_all(&dir);

    let mut slots = Vec::new();
    if let Ok(read_dir) = std::fs::read_dir(&dir) {
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

            let (is_auto, created_at) = match fs::read(&path)
                .ok()
                .and_then(|bytes| serde_json::from_slice::<SaveData>(&bytes).ok())
            {
                Some(data) => (data.is_auto, if data.created_at.is_empty() { "".into() } else { data.created_at }),
                None => (false, String::new()),
            };

            slots.push(SaveSlotMeta {
                is_auto,
                created_at,
                display_name,
                file_name,
            });
        }
    }

    slots.sort_by(|a, b| a.display_name.cmp(&b.display_name));
    slots_res.slots = slots;
}

// ---------- public helper: ensure current slot appears in list ----------
pub fn ensure_slot_in_list(slots_res: &mut SaveSlots, current: &CurrentSlot) {
    if let Some(curr_file) = &current.file_name {
        // already present?
        for s in &slots_res.slots {
            if &s.file_name == curr_file {
                return;
            }
        }
        // not present -> insert at head
        let display_name = curr_file.trim_end_matches(".json").to_string();
        slots_res.slots.insert(
            0,
            SaveSlotMeta {
                is_auto: false,
                created_at: String::new(),
                display_name,
                file_name: curr_file.clone(),
            },
        );
    }
}

/// 进入 InGame 时：
/// - 如果已经有存档列表，就选“最后一个存档”作为自动继续；
/// - 如果没有存档，就创建一个新的第 1 个存档名，例如 25.12.06.1
fn choose_autosave_or_new_slot(
    slots: Res<SaveSlots>,
    mut current: ResMut<CurrentSlot>,
    mut pending: ResMut<PendingLoad>,
) {
    if let Some(last) = slots.slots.last() {
        current.file_name = Some(last.file_name.clone());
        pending.file_name = Some(last.file_name.clone());
    } else {
        // 没有任何存档时，创建一个新的槽位名（只在内存里先记着，真正写文件在保存时）
        let display_name = generate_slot_display_name(1);
        let file_name = format!("{display_name}.json");
        current.file_name = Some(file_name.clone());
        pending.file_name = Some(file_name);
    }
}

/// 如果有 PendingLoad，就从对应文件读取 SaveData，并应用到玩家位置 & 血量
fn apply_pending_load(
    mut pending: ResMut<PendingLoad>,
    mut player_q: Query<(&mut Transform, &mut Health), With<Player>>,
) {
    let Some(file_name) = pending.file_name.take() else {
        return;
    };

    let path = slot_file_path(&file_name);
    let Ok(bytes) = fs::read(path) else {
        // 没读到文件，当作新游戏
        return;
    };

    let Ok(data) = serde_json::from_slice::<SaveData>(&bytes) else {
        return;
    };

    let Ok((mut tf, mut hp)) = player_q.single_mut() else {
        return;
    };

    tf.translation.x = data.player_x;
    tf.translation.y = data.player_y;
    hp.max = data.player_hp_max.max(1.0);
    hp.current = data.player_hp_current.clamp(0.0, hp.max);
}

/// 处理手动保存事件：
/// - ESC 菜单 / 主菜单中按“保存”按钮时，发出 ManualSaveEvent；
/// - 这里接到事件后，立即把当前玩家状态写入当前存档文件。
fn handle_manual_save_events(
    mut ev_save: MessageReader<ManualSaveEvent>,
    player_q: Query<(&Transform, &Health), With<Player>>,
    mut slots: ResMut<SaveSlots>,
    mut current: ResMut<CurrentSlot>,
) {
    if ev_save.is_empty() {
        return;
    }

    for ev in ev_save.read() {
        let Ok((tf, hp)) = player_q.single() else {
            error!("Manual save requested but no player entity exists.");
            continue;
        };

        if let Some(file_name) = &ev.file_name {
            let path = slot_file_path(file_name);
            let display_name = file_name.trim_end_matches(".json").to_string();
            let created_at = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();

            let data = SaveData {
                is_auto: false,
                display_name: display_name.clone(),
                created_at: created_at.clone(),
                player_x: tf.translation.x,
                player_y: tf.translation.y,
                player_hp_current: hp.current,
                player_hp_max: hp.max,
            };

            if let Ok(bytes) = serde_json::to_vec_pretty(&data) {
                if let Err(err) = fs::write(path, bytes) {
                    error!("Failed to write manual save {:?}: {err}", file_name);
                } else if !slots.slots.iter().any(|s| s.file_name == *file_name) {
                    slots.slots.insert(
                        0,
                        SaveSlotMeta {
                            is_auto: false,
                            created_at,
                            display_name,
                            file_name: file_name.clone(),
                        },
                    );
                }
            }

            current.file_name = Some(file_name.clone());
        } else {
            let now = Local::now();
            let y = now.year() % 100;
            let m = now.month();
            let d = now.day();

            let mut max_seq: u32 = 0;
            for slot in &slots.slots {
                if let Some(seq) = parse_seq_if_same_date(&slot.display_name, y, m, d) {
                    max_seq = max_seq.max(seq);
                }
            }

            let new_seq = max_seq + 1;
            let display_name = format!("{y}.{m}.{d}.{new_seq}");
            let file_name = format!("{display_name}.json");
            let created_at = now.format("%Y-%m-%d %H:%M:%S").to_string();

            let data = SaveData {
                is_auto: false,
                display_name: display_name.clone(),
                created_at: created_at.clone(),
                player_x: tf.translation.x,
                player_y: tf.translation.y,
                player_hp_current: hp.current,
                player_hp_max: hp.max,
            };

            let path = slot_file_path(&file_name);
            if let Ok(bytes) = serde_json::to_vec_pretty(&data) {
                if let Err(err) = fs::write(path, bytes) {
                    error!("Failed to write manual save {:?}: {err}", file_name);
                } else {
                    slots.slots.insert(
                        0,
                        SaveSlotMeta {
                            is_auto: false,
                            created_at,
                            display_name,
                            file_name: file_name.clone(),
                        },
                    );
                    current.file_name = Some(file_name);
                }
            }
        }
    }

    ev_save.clear();
}

/// 处理“读档”事件：
/// - UI 选择某一个存档（通过文件名），发送 LoadSlotEvent；
fn handle_load_slot_events(
    mut ev_load: MessageReader<LoadSlotEvent>,
    mut pending: ResMut<PendingLoad>,
    mut current: ResMut<CurrentSlot>,
) {
    for ev in ev_load.read() {
        pending.file_name = Some(ev.file_name.clone());
        current.file_name = Some(ev.file_name.clone());
    }

    ev_load.clear();
}

/// 自动保存：
/// - 使用 Bevy 的 Local<f32> 做一个简单计时器；
/// - 每 N 秒把当前玩家状态写回当前存档文件。
fn auto_save_every_n_seconds(
    time: Res<Time>,
    mut timer: Local<f32>,
    mut player_q: Query<(&Transform, &Health), With<Player>>,
    current: Res<CurrentSlot>,
) {
    let dt = time.delta_secs();
    *timer += dt;

    // 自动保存间隔（秒）
    let interval = 10.0;
    if *timer < interval {
        return;
    }
    *timer = 0.0;

    let Some(file_name) = &current.file_name else {
        return;
    };

    let Ok((tf, hp)) = player_q.single() else {
        return;
    };

    let display_name = file_name.trim_end_matches(".json").to_string();
    let created_at = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();

    let data = SaveData {
        is_auto: true,
        display_name,
        created_at,
        player_x: tf.translation.x,
        player_y: tf.translation.y,
        player_hp_current: hp.current,
        player_hp_max: hp.max,
    };

    let path = slot_file_path(file_name);
    if let Ok(bytes) = serde_json::to_vec_pretty(&data) {
        let _ = fs::write(path, bytes);
    }
}

fn parse_seq_if_same_date(name: &str, y: i32, m: u32, d: u32) -> Option<u32> {
    let parts: Vec<_> = name.split('.').collect();
    if parts.len() != 4 {
        return None;
    }

    let (yy, mm, dd, seq) = (
        parts[0].parse::<i32>().ok()?,
        parts[1].parse::<u32>().ok()?,
        parts[2].parse::<u32>().ok()?,
        parts[3].parse::<u32>().ok()?,
    );

    if yy == y && mm == m && dd == d {
        Some(seq)
    } else {
        None
    }
}
