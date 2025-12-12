use bevy::prelude::*;
use chrono::{Datelike, Local as ChronoLocal};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

use crate::health::Health;
use crate::movement::Player;
use crate::state::GameState;

/// 手动保存事件：file_name = Some("xxx.json") => 覆盖该文件，None => 新建
#[derive(Debug, Clone, Message)]
pub struct ManualSaveEvent {
    pub file_name: Option<String>,
    pub slot_index: Option<u32>,
}

/// 选择加载某一个存档槽位（UI “激活”后发送）
#[derive(Debug, Clone, Message)]
pub struct LoadSlotEvent {
    /// 要加载的存档文件名，例如 "25.12.06.1.json"
    pub file_name: String,
}

/// 单个存档槽的元数据（用于 UI 列表）
#[derive(Debug, Clone)]
pub struct SaveSlotMeta {
    /// 显示在 UI 上的名字，例如 "25.12.06.1"
    pub display_name: String,
    /// 实际文件名，例如 "25.12.06.1.json"
    pub file_name: String,
    /// 是否自动存档（仅用于 UI 显示）
    pub is_auto: bool,
    /// 可选：创建时间或显示信息
    pub created_at: String,
}

/// 所有存档槽列表（从磁盘扫描出来）
#[derive(Resource, Default, Debug)]
pub struct SaveSlots {
    pub slots: Vec<SaveSlotMeta>,
}

/// 当前使用中的存档文件名（自动保存 / 手动保存默认写到这里）
#[derive(Resource, Default, Debug)]
pub struct CurrentSlot {
    /// 文件名，例如 "25.12.06.1.json"
    pub file_name: Option<String>,
}

/// 待加载存档（只有点“激活存档”才会写入；加载成功/失败后会清空）
#[derive(Resource, Default, Debug)]
pub struct PendingLoad {
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

/// 自动存档间隔（秒）
const AUTOSAVE_INTERVAL_SECS: f32 = 60.0;

/// 存档系统插件
pub struct SavePlugin;

impl Plugin for SavePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SaveSlots>()
            .init_resource::<CurrentSlot>()
            .init_resource::<PendingLoad>()
            .add_message::<ManualSaveEvent>()
            .add_message::<LoadSlotEvent>()
            .add_systems(OnEnter(GameState::MainMenu), load_save_slots_from_disk);

        app.add_systems(Update, handle_load_slot_events);

        //  InGame 或 Paused 都允许“应用激活存档”
        app.add_systems(
            Update,
            apply_pending_load.run_if(in_state(GameState::InGame).or(in_state(GameState::Paused))),
        );

        //  InGame 或 Paused 都允许“手动保存”
        app.add_systems(
            Update,
            handle_manual_save_events
                .run_if(in_state(GameState::InGame).or(in_state(GameState::Paused))),
        );

        //  只在 InGame 自动保存（每分钟一次）
        app.add_systems(Update, auto_save_every_minute.run_if(in_state(GameState::InGame)));
    }
}

/// 存档目录：./saves
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

/// 生成格式为 `yy.MM.dd.n` 的显示名，比如 `25.12.06.1`
pub fn generate_slot_display_name(index: u32) -> String {
    let now = ChronoLocal::now();
    let yy = now.year() % 100;
    let mm = now.month();
    let dd = now.day();
    format!("{:02}.{:02}.{:02}.{}", yy, mm, dd, index)
}

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
            let is_auto = display_name.starts_with("auto_") || display_name == "autosave";

            slots.push(SaveSlotMeta {
                display_name,
                file_name,
                is_auto,
                created_at: String::new(),
            });
        }
    }

    // 按名字排序（日期.序号 这种格式基本能排出时间顺序）
    slots.sort_by(|a, b| a.display_name.cmp(&b.display_name));
    slots_res.slots = slots;
}

/// UI 点击“激活存档”后：
/// - 只设置 PendingLoad（真正读档在 apply_pending_load 里发生）
/// - 并把 CurrentSlot 指向该文件（之后自动存档写到这个槽）
fn handle_load_slot_events(mut ev: MessageReader<LoadSlotEvent>, mut pending: ResMut<PendingLoad>, mut current: ResMut<CurrentSlot>) {
    if ev.is_empty() {
        return;
    }

    for e in ev.read() {
        pending.file_name = Some(e.file_name.clone());
        current.file_name = Some(e.file_name.clone());
    }
}

/// 真正读档（只会在 PendingLoad 有值时触发）
/// 注意：如果玩家实体还没生成，就先不 take()，避免丢掉请求。
fn apply_pending_load(
    mut pending: ResMut<PendingLoad>,
    mut player_q: Query<(&mut Transform, &mut Health), With<Player>>,
) {
    if pending.file_name.is_none() {
        return;
    }

    // 玩家还不存在：等下一帧再试（不要清 pending）
    let Ok((mut tf, mut hp)) = player_q.single_mut() else {
        return;
    };

    let Some(file_name) = pending.file_name.take() else {
        return;
    };

    let path = slot_file_path(&file_name);
    let Ok(bytes) = fs::read(path) else {
        // 文件不存在就当作加载失败（不回退、不强制改位置）
        return;
    };

    let Ok(data) = serde_json::from_slice::<SaveData>(&bytes) else {
        return;
    };

    tf.translation.x = data.player_x;
    tf.translation.y = data.player_y;
    hp.max = data.hp_max.max(1.0);
    hp.current = data.hp_current.clamp(0.0, hp.max);
}

/// 手动保存：
/// - file_name=Some => 覆盖
/// - file_name=None => 新建当天序号存档
fn handle_manual_save_events(
    mut ev_save: MessageReader<ManualSaveEvent>,
    player_q: Query<(&Transform, &Health), With<Player>>,
    mut slots: ResMut<SaveSlots>,
    mut current: ResMut<CurrentSlot>,
) {
    if ev_save.is_empty() {
        return;
    }

    let Ok((tf, hp)) = player_q.single() else {
        return; // 主菜单没有玩家，直接忽略
    };

    for ev in ev_save.read() {
        if let Some(file_name) = &ev.file_name {
            write_save_to_file(file_name, tf, hp);

            if !slots.slots.iter().any(|s| &s.file_name == file_name) {
                slots.slots.push(SaveSlotMeta {
                    display_name: file_name.trim_end_matches(".json").to_string(),
                    file_name: file_name.clone(),
                    is_auto: false,
                    created_at: ChronoLocal::now().format("%Y-%m-%d %H:%M:%S").to_string(),
                });
                slots.slots.sort_by(|a, b| a.display_name.cmp(&b.display_name));
            }

            current.file_name = Some(file_name.clone());
        } else {
            // 新建：找出当天最大序号 + 1
            let now = ChronoLocal::now();
            let y = (now.year() % 100) as u32;
            let m = now.month();
            let d = now.day();

            let mut max_seq: u32 = 0;
            for slot in &slots.slots {
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
                if yy == Some(y) && mm == Some(m) && dd == Some(d) {
                    if let Some(s) = seq {
                        max_seq = max_seq.max(s);
                    }
                }
            }

            let new_seq = max_seq + 1;
            let display_name = format!("{:02}.{:02}.{:02}.{}", y, m, d, new_seq);
            let file_name = format!("{display_name}.json");

            write_save_to_file(&file_name, tf, hp);

            slots.slots.push(SaveSlotMeta {
                display_name,
                file_name: file_name.clone(),
                is_auto: false,
                created_at: ChronoLocal::now().format("%Y-%m-%d %H:%M:%S").to_string(),
            });
            slots.slots.sort_by(|a, b| a.display_name.cmp(&b.display_name));

            current.file_name = Some(file_name);
        }
    }
}

fn write_save_to_file(file_name: &str, tf: &Transform, hp: &Health) {
    let data = SaveData {
        player_x: tf.translation.x,
        player_y: tf.translation.y,
        hp_current: hp.current,
        hp_max: hp.max,
    };

    let path = slot_file_path(file_name);
    if let Ok(bytes) = serde_json::to_vec_pretty(&data) {
        if let Err(e) = fs::write(&path, bytes) {
            error!("Failed to write save to {:?}: {}", path, e);
        }
    }
}

/// 自动存档：每 60 秒一次（如果 CurrentSlot 为空，就写到 autosave.json）
/// Bevy 官方 Timer 用法：tick(delta) + just_finished()
fn auto_save_every_minute(
    time: Res<Time>,
    mut timer: Local<Option<Timer>>,
    player_q: Query<(&Transform, &Health), With<Player>>,
    mut current: ResMut<CurrentSlot>,
    mut slots: ResMut<SaveSlots>,
) {
    if timer.is_none() {
        *timer = Some(Timer::from_seconds(AUTOSAVE_INTERVAL_SECS, TimerMode::Repeating));
    }

    let t = timer.as_mut().unwrap();
    if !t.tick(time.delta()).just_finished() {
        return;
    }

    let Ok((tf, hp)) = player_q.single() else {
        return;
    };

    let file_name = current
        .file_name
        .clone()
        .unwrap_or_else(|| "autosave.json".to_string());

    write_save_to_file(&file_name, tf, hp);

    // 确保 UI 列表能看到 autosave
    if !slots.slots.iter().any(|s| s.file_name == file_name) {
        slots.slots.push(SaveSlotMeta {
            display_name: file_name.trim_end_matches(".json").to_string(),
            file_name: file_name.clone(),
            is_auto: true,
            created_at: ChronoLocal::now().format("%Y-%m-%d %H:%M:%S").to_string(),
        });
        slots.slots.sort_by(|a, b| a.display_name.cmp(&b.display_name));
    }

    // 如果之前没有 current slot，就把 autosave 设为当前
    if current.file_name.is_none() {
        current.file_name = Some(file_name);
    }
}
