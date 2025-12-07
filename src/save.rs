use bevy::prelude::*;
use chrono::Datelike; // 提供 year()/month()/day()
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

use crate::health::Health;
use crate::movement::Player;
use crate::state::GameState;

/// 手动保存事件：file_name = Some("xxx.json") => 覆盖该文件，None => 新建
#[derive(Debug, Clone)]
pub struct ManualSaveEvent {
    pub file_name: Option<String>,
}

/// 选择加载某一个存档槽位（UI 激活后发送）
#[derive(Debug, Clone)]
pub struct LoadSlotEvent {
    /// 要加载的存档文件名，例如 "25.12.06.1.json"
    pub file_name: String,
}

// 手动实现 Message trait，让它们能被 MessageReader / add_event 使用
impl Message for ManualSaveEvent {}
impl Message for LoadSlotEvent {}

/// 单个存档槽的元数据（用于 UI 列表）
#[derive(Debug, Clone)]
pub struct SaveSlotMeta {
    /// 显示在 UI 上的名字，例如 "25.12.06.1"
    pub display_name: String,
    /// 实际文件名，例如 "25.12.06.1.json"
    pub file_name: String,
    /// 是否自动存档
    pub is_auto: bool,
    /// 可选：创建时间或显示信息
    pub created_at: String,
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
    pub player_x: f32,
    pub player_y: f32,
    pub hp_current: f32,
    pub hp_max: f32,
}

/// 存档系统插件
pub struct SavePlugin;

impl Plugin for SavePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SaveSlots>()
            .init_resource::<CurrentSlot>()
            .init_resource::<PendingLoad>()
            // 注册“事件类型”（Message）
            .add_event::<ManualSaveEvent>()
            .add_event::<LoadSlotEvent>()
            // 回到主菜单时，重新扫描硬盘上的所有存档
            .add_systems(OnEnter(GameState::MainMenu), load_save_slots_from_disk)
            // 进入游戏时：决定要用哪个存档，并尝试加载
            .add_systems(OnEnter(GameState::InGame), (choose_autosave_or_new_slot, apply_pending_load));

        // 自动保存 / 以及手动保存、读档处理 分别注册到 Update，并用 run_if 控制
        app.add_systems(Update, auto_save_every_n_seconds.run_if(in_state(GameState::InGame)));
        app.add_systems(Update, handle_manual_save_events.run_if(in_state(GameState::InGame)));
        app.add_systems(Update, handle_load_slot_events.run_if(in_state(GameState::InGame)));

        // 暂停菜单也允许手动存档（致使用户在暂停时点“保存”）
        app.add_systems(Update, handle_manual_save_events.run_if(in_state(GameState::Paused)));
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

/// 生成格式为 `yy.MM.dd.n` 的显示名，比如 `25.12.06.1`
/// year 用后两位（2025 -> 25）
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

/// Scan ./saves and fill SaveSlots (public for UI to refresh)
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
            // push minimal meta; created_at can be filled later if you want
            slots.push(SaveSlotMeta {
                display_name,
                file_name,
                is_auto: false,
                created_at: String::new(),
            });
        }
    }

    // 可以按名字排序一下（大致就是按时间 / 序号）
    slots.sort_by(|a, b| a.display_name.cmp(&b.display_name));
    slots_res.slots = slots;
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
    hp.max = data.hp_max.max(1.0);
    hp.current = data.hp_current.clamp(0.0, hp.max);
}

/// 处理手动保存事件：
/// - ESC 菜单 / 主菜单中按“保存”按钮时，发出 ManualSaveEvent；
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
            continue;
        };

        if let Some(file_name) = &ev.file_name {
            // 覆盖指定文件（手动覆盖现有存档）
            let data = SaveData {
                player_x: tf.translation.x,
                player_y: tf.translation.y,
                hp_current: hp.current,
                hp_max: hp.max,
            };
            let path = slot_file_path(file_name);
            if let Ok(bytes) = serde_json::to_vec_pretty(&data) {
                if let Err(e) = fs::write(&path, bytes) {
                    error!("Failed to write manual save to {:?}: {}", path, e);
                } else {
                    // 保证内存 slots 包含此文件
                    if !slots.slots.iter().any(|s| &s.file_name == file_name) {
                        slots.slots.insert(0, SaveSlotMeta {
                            display_name: file_name.trim_end_matches(".json").to_string(),
                            file_name: file_name.clone(),
                            is_auto: false,
                            created_at: chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
                        });
                    }
                    current.file_name = Some(file_name.clone());
                }
            }
        } else {
            // 新建一个手动存档（生成当天序号）
            let now = chrono::Local::now();
            let y = (now.year() % 100) as u32;
            let m = now.month();
            let d = now.day();

            // 找出当天已有的最大序号
            let mut max_seq: u32 = 0;
            for slot in &slots.slots {
                // 试着解析像 "25.12.06.3" 的尾号
                if let Some(parts) = slot.display_name.split('.').collect::<Vec<_>>().as_slice().get(3) {
                    if let Ok(seq) = parts.parse::<u32>() {
                        let year = slot.display_name.split('.').collect::<Vec<_>>().get(0).and_then(|s| s.parse::<u32>().ok()).unwrap_or(0);
                        let month = slot.display_name.split('.').collect::<Vec<_>>().get(1).and_then(|s| s.parse::<u32>().ok()).unwrap_or(0);
                        let day = slot.display_name.split('.').collect::<Vec<_>>().get(2).and_then(|s| s.parse::<u32>().ok()).unwrap_or(0);
                        if year == y && month == m && day == d {
                            if seq > max_seq {
                                max_seq = seq;
                            }
                        }
                    }
                }
            }
            let new_seq = max_seq + 1;
            let display_name = format!("{:02}.{:02}.{:02}.{}", y, m, d, new_seq);
            let file_name = format!("{display_name}.json");
            let created_at = now.format("%Y-%m-%d %H:%M:%S").to_string();

            let data = SaveData {
                player_x: tf.translation.x,
                player_y: tf.translation.y,
                hp_current: hp.current,
                hp_max: hp.max,
            };
            let path = slot_file_path(&file_name);
            if let Ok(bytes) = serde_json::to_vec_pretty(&data) {
                if let Err(e) = fs::write(&path, bytes) {
                    error!("Failed to create manual save {:?}: {}", path, e);
                } else {
                    slots.slots.insert(0, SaveSlotMeta {
                        display_name: display_name.clone(),
                        file_name: file_name.clone(),
                        is_auto: false,
                        created_at,
                    });
                    current.file_name = Some(file_name);
                }
            }
        }
    }

    ev_save.clear();
}

/// 处理“读档”事件：
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

/// 自动保存：使用显式 Bevy Local 类型以避免与 chrono::Local 冲突
fn auto_save_every_n_seconds(
    time: Res<Time>,
    mut timer: bevy::ecs::system::Local<f32>,
    mut player_q: Query<(&Transform, &Health), With<Player>>,
    current: Res<CurrentSlot>,
) {
    let dt = time.delta_seconds();
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

    let data = SaveData {
        player_x: tf.translation.x,
        player_y: tf.translation.y,
        hp_current: hp.current,
        hp_max: hp.max,
    };

    let path = slot_file_path(file_name);
    if let Ok(bytes) = serde_json::to_vec_pretty(&data) {
        let _ = fs::write(path, bytes);
    }
}
// ------------------ end replacement for src/save.rs ------------------
