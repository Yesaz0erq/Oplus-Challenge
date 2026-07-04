use crate::equipment::ItemId;
use crate::skills_pool::{SkillId, SkillRarity};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub enum Language {
    #[default]
    ZhCn,
    EnUs,
}

impl Language {
    pub fn cycle(self, dir: i32) -> Self {
        let list = [Language::ZhCn, Language::EnUs];
        let idx = list.iter().position(|x| *x == self).unwrap_or(0);
        if dir >= 0 {
            list[(idx + 1) % list.len()]
        } else {
            list[(idx + list.len() - 1) % list.len()]
        }
    }
}

pub struct L10n;

#[allow(dead_code)]
impl L10n {
    pub fn language_name(lang: Language) -> &'static str {
        match lang {
            Language::ZhCn => "中文",
            Language::EnUs => "English",
        }
    }

    pub fn main_menu_start(lang: Language) -> &'static str {
        match lang {
            Language::ZhCn => "开始游戏",
            Language::EnUs => "Start Game",
        }
    }
    pub fn main_menu_save(lang: Language) -> &'static str {
        match lang {
            Language::ZhCn => "存档",
            Language::EnUs => "Saves",
        }
    }
    pub fn main_menu_settings(lang: Language) -> &'static str {
        match lang {
            Language::ZhCn => "设置",
            Language::EnUs => "Settings",
        }
    }
    pub fn main_menu_exit(lang: Language) -> &'static str {
        match lang {
            Language::ZhCn => "退出",
            Language::EnUs => "Exit",
        }
    }

    pub fn main_menu_subtitle(lang: Language) -> &'static str {
        match lang {
            Language::ZhCn => "沙海与回响",
            Language::EnUs => "Sands & Echoes",
        }
    }

    pub fn pause_resume(lang: Language) -> &'static str {
        match lang {
            Language::ZhCn => "继续游戏",
            Language::EnUs => "Resume",
        }
    }
    pub fn pause_back_to_menu(lang: Language) -> &'static str {
        match lang {
            Language::ZhCn => "返回主菜单",
            Language::EnUs => "Back to Title",
        }
    }

    pub fn dialogue_npc_name(lang: Language) -> &'static str {
        match lang {
            Language::ZhCn => "沙海旅人",
            Language::EnUs => "Wanderer",
        }
    }

    pub fn dialogue_advance_hint(lang: Language) -> &'static str {
        match lang {
            Language::ZhCn => "E 继续",
            Language::EnUs => "E Continue",
        }
    }

    pub fn dialogue_close_hint(lang: Language) -> &'static str {
        match lang {
            Language::ZhCn => "E 结束对话",
            Language::EnUs => "E End Dialogue",
        }
    }

    pub fn dialogue_page(lang: Language, page: usize) -> &'static str {
        match (lang, page) {
            (Language::ZhCn, 0) => "风沙会掩埋旧路，但也会替愿意前行的人指明新路。",
            (Language::ZhCn, 1) => {
                "继续向东之前，先把武器和技能准备好。荒地里的东西不会给你第二次犹豫的机会。"
            }
            (Language::ZhCn, 2) => {
                "如果累了，就回来找我。我会一直守着这片沙地，替迷路的人点一盏灯。"
            }
            (Language::EnUs, 0) => {
                "The sand buries old roads, but it also reveals new ones to those who keep moving."
            }
            (Language::EnUs, 1) => {
                "Ready your weapon and skills before heading east. The wasteland rarely gives a second chance."
            }
            (Language::EnUs, 2) => {
                "If you grow tired, come back and find me. I will keep a light here for the lost."
            }
            _ => "",
        }
    }

    pub fn settings_title(lang: Language) -> &'static str {
        match lang {
            Language::ZhCn => "设置",
            Language::EnUs => "Settings",
        }
    }
    pub fn settings_resolution(lang: Language) -> &'static str {
        match lang {
            Language::ZhCn => "分辨率",
            Language::EnUs => "Resolution",
        }
    }
    pub fn settings_fullscreen(lang: Language) -> &'static str {
        match lang {
            Language::ZhCn => "全屏",
            Language::EnUs => "Fullscreen",
        }
    }
    pub fn settings_volume(lang: Language) -> &'static str {
        match lang {
            Language::ZhCn => "音量",
            Language::EnUs => "Volume",
        }
    }
    pub fn settings_language(lang: Language) -> &'static str {
        match lang {
            Language::ZhCn => "语言",
            Language::EnUs => "Language",
        }
    }
    pub fn settings_toggle(lang: Language) -> &'static str {
        match lang {
            Language::ZhCn => "切换",
            Language::EnUs => "Toggle",
        }
    }
    pub fn settings_apply(lang: Language) -> &'static str {
        match lang {
            Language::ZhCn => "应用",
            Language::EnUs => "Apply",
        }
    }
    pub fn settings_back(lang: Language) -> &'static str {
        match lang {
            Language::ZhCn => "返回",
            Language::EnUs => "Back",
        }
    }
    pub fn settings_on(lang: Language) -> &'static str {
        match lang {
            Language::ZhCn => "开",
            Language::EnUs => "On",
        }
    }
    pub fn settings_off(lang: Language) -> &'static str {
        match lang {
            Language::ZhCn => "关",
            Language::EnUs => "Off",
        }
    }
    pub fn debug_menu_title(lang: Language) -> &'static str {
        match lang {
            Language::ZhCn => "调试模式",
            Language::EnUs => "Debug Menu",
        }
    }
    pub fn debug_hp_boost(lang: Language) -> &'static str {
        match lang {
            Language::ZhCn => "血量上限 10 万",
            Language::EnUs => "Max HP 100000",
        }
    }
    pub fn debug_noclip(lang: Language) -> &'static str {
        match lang {
            Language::ZhCn => "穿墙（关闭墙体碰撞）",
            Language::EnUs => "No Clip (Disable Wall Collision)",
        }
    }
    pub fn debug_no_cooldown(lang: Language) -> &'static str {
        match lang {
            Language::ZhCn => "取消冷却",
            Language::EnUs => "No Cooldown",
        }
    }
    pub fn debug_menu_hotkey_hint(lang: Language) -> &'static str {
        match lang {
            Language::ZhCn => "Alt+C 打开/关闭，ESC 关闭",
            Language::EnUs => "Alt+C to toggle, ESC to close",
        }
    }

    pub fn save_title(lang: Language) -> &'static str {
        match lang {
            Language::ZhCn => "存档",
            Language::EnUs => "Saves",
        }
    }
    pub fn save_manual(lang: Language) -> &'static str {
        match lang {
            Language::ZhCn => "手动保存",
            Language::EnUs => "Manual Save",
        }
    }
    pub fn save_load_selected(lang: Language) -> &'static str {
        match lang {
            Language::ZhCn => "载入选中存档",
            Language::EnUs => "Load Selected",
        }
    }
    pub fn save_delete_selected(lang: Language) -> &'static str {
        match lang {
            Language::ZhCn => "删除选中存档",
            Language::EnUs => "Delete Selected",
        }
    }
    pub fn save_empty(lang: Language) -> &'static str {
        match lang {
            Language::ZhCn => "暂无存档（请先手动保存一次）",
            Language::EnUs => "No saves yet (create a manual save first).",
        }
    }
    pub fn save_auto_tag(lang: Language) -> &'static str {
        match lang {
            Language::ZhCn => "自动",
            Language::EnUs => "Auto",
        }
    }
    pub fn save_selected_tag(lang: Language) -> &'static str {
        match lang {
            Language::ZhCn => "已选中",
            Language::EnUs => "Selected",
        }
    }
    pub fn save_entry_prefix(lang: Language) -> &'static str {
        match lang {
            Language::ZhCn => "存档",
            Language::EnUs => "Save",
        }
    }

    pub fn game_over_title(lang: Language) -> &'static str {
        match lang {
            Language::ZhCn => "游戏失败",
            Language::EnUs => "Game Over",
        }
    }
    pub fn game_over_desc(lang: Language) -> &'static str {
        match lang {
            Language::ZhCn => "请选择一个【手动存档】重新开始（不会使用自动存档）",
            Language::EnUs => "Choose a manual save to restart (autosaves are not used).",
        }
    }
    pub fn game_over_no_manual_saves(lang: Language) -> &'static str {
        match lang {
            Language::ZhCn => "暂无手动存档：请先在游戏内打开“存档面板”进行手动保存。",
            Language::EnUs => "No manual saves. Open the save panel in-game and create one first.",
        }
    }
    pub fn game_over_load_restart(lang: Language) -> &'static str {
        match lang {
            Language::ZhCn => "加载并重新开始",
            Language::EnUs => "Load and Restart",
        }
    }
    pub fn game_over_back_to_title(lang: Language) -> &'static str {
        match lang {
            Language::ZhCn => "返回标题界面",
            Language::EnUs => "Back to Title",
        }
    }

    pub fn equipment_inventory(lang: Language) -> &'static str {
        match lang {
            Language::ZhCn => "背包",
            Language::EnUs => "Inventory",
        }
    }
    pub fn equipment_equipped_weapon_header(lang: Language) -> &'static str {
        match lang {
            Language::ZhCn => "已装备武器",
            Language::EnUs => "Equipped Weapon",
        }
    }
    pub fn equipment_page_label(
        lang: Language,
        current: usize,
        total: usize,
        _slots: usize,
    ) -> String {
        match lang {
            Language::ZhCn => format!("第 {}/{} 页", current, total),
            Language::EnUs => format!("Page {}/{}", current, total),
        }
    }
    pub fn equipment_prev_page(lang: Language) -> &'static str {
        match lang {
            Language::ZhCn => "上一页",
            Language::EnUs => "Prev",
        }
    }
    pub fn equipment_next_page(lang: Language) -> &'static str {
        match lang {
            Language::ZhCn => "下一页",
            Language::EnUs => "Next",
        }
    }
    pub fn equipment_item_details(lang: Language) -> &'static str {
        match lang {
            Language::ZhCn => "物品详情",
            Language::EnUs => "Item Details",
        }
    }
    pub fn equipment_hover_compare_hint(lang: Language) -> &'static str {
        match lang {
            Language::ZhCn => "将鼠标悬浮在背包物品上，鼠标旁会显示对比信息。",
            Language::EnUs => "Hover an inventory item to show comparison near the cursor.",
        }
    }
    pub fn close(lang: Language) -> &'static str {
        match lang {
            Language::ZhCn => "关闭",
            Language::EnUs => "Close",
        }
    }
    pub fn equipped_tag(lang: Language) -> &'static str {
        match lang {
            Language::ZhCn => "已装备",
            Language::EnUs => "Equipped",
        }
    }
    pub fn stat_damage(lang: Language) -> &'static str {
        match lang {
            Language::ZhCn => "伤害",
            Language::EnUs => "DMG",
        }
    }
    pub fn stat_cooldown(lang: Language) -> &'static str {
        match lang {
            Language::ZhCn => "冷却",
            Language::EnUs => "CD",
        }
    }
    pub fn stat_range(lang: Language) -> &'static str {
        match lang {
            Language::ZhCn => "射程",
            Language::EnUs => "Range",
        }
    }
    pub fn equipment_weapon_summary(
        lang: Language,
        weapon_name: &str,
        dmg: f32,
        cooldown: f32,
        range: f32,
    ) -> String {
        format!(
            "{}: {} ({})\n{}: {:.0}\n{}: {:.2}\n{}: {:.0}",
            Self::equipment_equipped_weapon_header(lang),
            weapon_name,
            Self::equipped_tag(lang),
            Self::stat_damage(lang),
            dmg,
            Self::stat_cooldown(lang),
            cooldown,
            Self::stat_range(lang),
            range
        )
    }
    pub fn memory_label(lang: Language) -> &'static str {
        match lang {
            Language::ZhCn => "内存",
            Language::EnUs => "Memory",
        }
    }
    pub fn memory_base_name(lang: Language) -> &'static str {
        match lang {
            Language::ZhCn => "基础存储组件",
            Language::EnUs => "Basic Storage Module",
        }
    }
    pub fn memory_capacity_label(lang: Language) -> &'static str {
        match lang {
            Language::ZhCn => "可携带技能",
            Language::EnUs => "Skill Capacity",
        }
    }
    pub fn memory_level_label(lang: Language) -> &'static str {
        match lang {
            Language::ZhCn => "等级",
            Language::EnUs => "Level",
        }
    }
    pub fn memory_summary(lang: Language, name: &str, level: u32, capacity: usize) -> String {
        match lang {
            Language::ZhCn => format!(
                "{}: {}\n{}: {}  {}: {}",
                Self::memory_label(lang),
                name,
                Self::memory_level_label(lang),
                level,
                Self::memory_capacity_label(lang),
                capacity
            ),
            Language::EnUs => format!(
                "{}: {}\n{}: {}  {}: {}",
                Self::memory_label(lang),
                name,
                Self::memory_level_label(lang),
                level,
                Self::memory_capacity_label(lang),
                capacity
            ),
        }
    }
    pub fn hp_atk(lang: Language, hp_cur: f32, hp_max: f32, atk: f32) -> String {
        match lang {
            Language::ZhCn => format!("生命: {:.0}/{:.0}   攻击: {:.0}", hp_cur, hp_max, atk),
            Language::EnUs => format!("HP: {:.0}/{:.0}   ATK: {:.0}", hp_cur, hp_max, atk),
        }
    }
    pub fn hp_short(lang: Language, hp_cur: f32, hp_max: f32) -> String {
        match lang {
            Language::ZhCn => format!("生命: {:.0}/{:.0}", hp_cur, hp_max),
            Language::EnUs => format!("HP: {:.0}/{:.0}", hp_cur, hp_max),
        }
    }
    pub fn skills_hp_label(lang: Language) -> &'static str {
        match lang {
            Language::ZhCn => "生命",
            Language::EnUs => "HP",
        }
    }
    pub fn skill_name(lang: Language, id: SkillId) -> &'static str {
        match (lang, id) {
            (Language::ZhCn, SkillId::Dash) => "冲刺",
            (Language::ZhCn, SkillId::Slash) => "斩击",
            (Language::ZhCn, SkillId::Fireball) => "火球术",
            (Language::ZhCn, SkillId::LightWave) => "光波",
            (Language::EnUs, SkillId::Dash) => "Dash",
            (Language::EnUs, SkillId::Slash) => "Slash",
            (Language::EnUs, SkillId::Fireball) => "Fireball",
            (Language::EnUs, SkillId::LightWave) => "Light Wave",
        }
    }
    pub fn skill_effect_desc(lang: Language, id: SkillId) -> &'static str {
        match (lang, id) {
            (Language::ZhCn, SkillId::Dash) => "短时间高速位移。",
            (Language::ZhCn, SkillId::Slash) => "向前方释放扇形斩击，造成近战范围伤害。",
            (Language::ZhCn, SkillId::Fireball) => "向光标发射大型火球，命中敌人或墙体时消失。",
            (Language::ZhCn, SkillId::LightWave) => "沿直线发射贯穿光波，可穿墙并命中多名敌人。",
            (Language::EnUs, SkillId::Dash) => "Dash forward with high speed for a short duration.",
            (Language::EnUs, SkillId::Slash) => {
                "Release a forward slash that deals melee area damage."
            }
            (Language::EnUs, SkillId::Fireball) => {
                "Launch a large fireball toward the cursor. It disappears on enemy or wall hit."
            }
            (Language::EnUs, SkillId::LightWave) => {
                "Emit a piercing light wave in a straight line. It can pass through walls."
            }
        }
    }
    pub fn skill_rarity(lang: Language, rarity: SkillRarity) -> &'static str {
        match (lang, rarity) {
            (Language::ZhCn, SkillRarity::Common) => "普通",
            (Language::EnUs, SkillRarity::Common) => "Common",
        }
    }
    pub fn skill_backpack_title(lang: Language) -> &'static str {
        match lang {
            Language::ZhCn => "技能背包",
            Language::EnUs => "Skill Backpack",
        }
    }
    pub fn skill_backpack_page(lang: Language, current: usize, total: usize) -> String {
        match lang {
            Language::ZhCn => format!("第 {}/{} 页", current, total),
            Language::EnUs => format!("Page {}/{}", current, total),
        }
    }
    pub fn skill_backpack_capacity(lang: Language, used: usize, cap: usize) -> String {
        match lang {
            Language::ZhCn => format!("携带技能: {used}/{cap}"),
            Language::EnUs => format!("Carried Skills: {used}/{cap}"),
        }
    }
    pub fn skill_card_damage(lang: Language) -> &'static str {
        match lang {
            Language::ZhCn => "伤害",
            Language::EnUs => "Damage",
        }
    }
    pub fn skill_card_cooldown(lang: Language) -> &'static str {
        match lang {
            Language::ZhCn => "冷却",
            Language::EnUs => "Cooldown",
        }
    }
    pub fn skill_card_rarity(lang: Language) -> &'static str {
        match lang {
            Language::ZhCn => "稀有度",
            Language::EnUs => "Rarity",
        }
    }
    pub fn skill_card_effect(lang: Language) -> &'static str {
        match lang {
            Language::ZhCn => "效果",
            Language::EnUs => "Effect",
        }
    }
    pub fn skill_parse_name(lang: Language) -> &'static str {
        match lang {
            Language::ZhCn => "解析",
            Language::EnUs => "Analyze",
        }
    }
    pub fn skill_parse_ready(lang: Language) -> &'static str {
        match lang {
            Language::ZhCn => "就绪",
            Language::EnUs => "Ready",
        }
    }
    pub fn skill_parse_selecting(lang: Language) -> &'static str {
        match lang {
            Language::ZhCn => "选择目标(R 取消)",
            Language::EnUs => "Select target (R to cancel)",
        }
    }
    pub fn skill_parse_popup_title(lang: Language) -> &'static str {
        match lang {
            Language::ZhCn => "技能解析结果",
            Language::EnUs => "Skill Analysis Result",
        }
    }
    pub fn skill_parse_new_skill_desc(lang: Language, skill_name: &str) -> String {
        match lang {
            Language::ZhCn => format!("解析到新技能：{skill_name}\n可选择保存到技能背包。"),
            Language::EnUs => {
                format!("New skill analyzed: {skill_name}\nYou can save it to your skill backpack.")
            }
        }
    }
    pub fn skill_parse_owned_skill_desc(lang: Language, skill_name: &str) -> String {
        match lang {
            Language::ZhCn => format!(
                "已拥有技能：{skill_name}\n本次解析生成了±25%浮动的新数值。可选择替换原技能。"
            ),
            Language::EnUs => format!(
                "Already owned skill: {skill_name}\nThis analysis rolled new stats within about +/-25%. Choose whether to replace."
            ),
        }
    }
    pub fn skill_parse_old_values(lang: Language) -> &'static str {
        match lang {
            Language::ZhCn => "当前数值",
            Language::EnUs => "Current Values",
        }
    }
    pub fn skill_parse_new_values(lang: Language) -> &'static str {
        match lang {
            Language::ZhCn => "新数值",
            Language::EnUs => "New Values",
        }
    }
    pub fn skill_parse_save_to_bag(lang: Language) -> &'static str {
        match lang {
            Language::ZhCn => "保存到背包",
            Language::EnUs => "Save to Bag",
        }
    }
    pub fn skill_parse_discard(lang: Language) -> &'static str {
        match lang {
            Language::ZhCn => "放弃",
            Language::EnUs => "Discard",
        }
    }
    pub fn skill_parse_replace(lang: Language) -> &'static str {
        match lang {
            Language::ZhCn => "替换原技能",
            Language::EnUs => "Replace",
        }
    }
    pub fn skill_parse_keep_old(lang: Language) -> &'static str {
        match lang {
            Language::ZhCn => "保留原技能",
            Language::EnUs => "Keep Old",
        }
    }
    pub fn skill_parse_bag_full(lang: Language) -> &'static str {
        match lang {
            Language::ZhCn => "技能背包已满，无法保存新技能。",
            Language::EnUs => "Skill backpack is full. New skill cannot be saved.",
        }
    }
    pub fn skill_backpack_empty(lang: Language) -> &'static str {
        match lang {
            Language::ZhCn => "空槽位",
            Language::EnUs => "Empty Slot",
        }
    }
    pub fn skill_backpack_capacity_label(lang: Language) -> &'static str {
        match lang {
            Language::ZhCn => "携带技能",
            Language::EnUs => "Carried Skills",
        }
    }
    pub fn skill_backpack_selected(lang: Language) -> &'static str {
        match lang {
            Language::ZhCn => "已携带",
            Language::EnUs => "Equipped",
        }
    }
    pub fn item_name(lang: Language, id: ItemId) -> &'static str {
        match (lang, id) {
            (Language::ZhCn, ItemId::RustySword) => "生锈短剑",
            (Language::ZhCn, ItemId::MagicWand) => "法杖",
            (Language::ZhCn, ItemId::HunterBow) => "猎弓",
            (Language::EnUs, ItemId::RustySword) => "Rusty Sword",
            (Language::EnUs, ItemId::MagicWand) => "Magic Wand",
            (Language::EnUs, ItemId::HunterBow) => "Hunter Bow",
        }
    }
    pub fn kind_melee(lang: Language) -> &'static str {
        match lang {
            Language::ZhCn => "近战",
            Language::EnUs => "Melee",
        }
    }
    pub fn kind_ranged(lang: Language) -> &'static str {
        match lang {
            Language::ZhCn => "远程",
            Language::EnUs => "Ranged",
        }
    }
}
