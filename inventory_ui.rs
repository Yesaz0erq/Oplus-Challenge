// src/inventory_ui.rs
use bevy::prelude::ImageNode;
use bevy::prelude::*;
use bevy::ui::{BorderColor, BorderRadius, Display, FocusPolicy, RepeatedGridTrack};

use crate::equipment::{EquipmentSet, EquippedItems, ItemDatabase, ItemId};
use crate::inventory::{Inventory, ItemStack};
use crate::movement::Player;

/// Inventory UI Plugin
pub struct InventoryUiPlugin;

#[derive(Resource)]
pub struct InventoryUiConfig {
    pub toggle_key: KeyCode,
    pub cols: usize,
    pub rows: usize,
    pub slot_px: f32,
}
impl Default for InventoryUiConfig {
    fn default() -> Self {
        Self {
            // 按 B 打开背包
            toggle_key: KeyCode::KeyB,
            cols: 10,
            rows: 4, // 每页 40 格
            slot_px: 48.0,
        }
    }
}

#[derive(Resource, Default)]
pub struct InventoryUiState {
    pub open: bool,
    pub page: usize,
    pub selected: Option<usize>, // 绝对 slot index
}

#[derive(Component)]
struct InventoryUiRoot;

#[derive(Component)]
struct SlotButton {
    slot_index: usize,
}

#[derive(Component)]
struct PrevPageBtn;
#[derive(Component)]
struct NextPageBtn;

/// UI -> logic messages (backpack slot clicked / page change)
#[derive(Message, Clone, Copy, Debug)]
pub struct InventorySlotClickMsg {
    pub slot_index: usize,
}

#[derive(Message, Clone, Copy, Debug)]
pub struct InventoryPageMsg {
    pub delta: i32,
}

impl Plugin for InventoryUiPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<InventoryUiConfig>()
            .init_resource::<InventoryUiState>()
            .add_message::<InventorySlotClickMsg>()
            .add_message::<InventoryPageMsg>()
            .add_systems(Update, toggle_inventory_ui)
            .add_systems(Update, handle_inventory_ui_interactions)
            .add_systems(Update, apply_inventory_ui_messages)
            .add_systems(Update, rebuild_inventory_ui_on_change);
    }
}

/// 当按键切换背包时打开/关闭
fn toggle_inventory_ui(
    keyboard: Res<ButtonInput<KeyCode>>,
    cfg: Res<InventoryUiConfig>,
    mut state: ResMut<InventoryUiState>,
) {
    if keyboard.just_pressed(cfg.toggle_key) {
        state.open = !state.open;
    }
}

/// 重建背包 UI：现在需要 EquipmentSet/EquippedItems + Inventory
fn rebuild_inventory_ui_on_change(
    mut commands: Commands,
    cfg: Res<InventoryUiConfig>,
    state: Res<InventoryUiState>,
    q_root: Query<Entity, With<InventoryUiRoot>>,
    q_player_inv: Query<&Inventory, With<Player>>,
    q_equip: Query<(&EquippedItems, &EquipmentSet), With<Player>>,
    asset_server: Res<AssetServer>,
) {
    // 清旧 UI（如果有）
    if let Ok(root) = q_root.single() {
        commands.entity(root).try_despawn();
    }

    if !state.open {
        return;
    }

    let Ok(inv) = q_player_inv.single() else {
        return;
    };
    let Ok((equipped, equip_set)) = q_equip.single() else {
        return;
    };

    spawn_inventory_ui(
        &mut commands,
        &asset_server,
        &cfg,
        &*state,
        inv,
        equipped,
        equip_set,
    );
}

/// 生成界面（含左侧装备属性面板 + 右侧背包格子）
fn spawn_inventory_ui(
    commands: &mut Commands,
    asset_server: &AssetServer,
    cfg: &InventoryUiConfig,
    state: &InventoryUiState,
    inv: &Inventory,
    equipped: &EquippedItems,
    equip_set: &EquipmentSet,
) {
    let font: Handle<Font> = asset_server.load("fonts/YuFanLixing.otf");

    let page_size = cfg.cols * cfg.rows;
    let page_count = (inv.slot_count() + page_size - 1) / page_size;
    let page = state.page.min(page_count.saturating_sub(1));
    let start = page * page_size;
    let end = (start + page_size).min(inv.slot_count());

    commands
        .spawn((
            InventoryUiRoot,
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                position_type: PositionType::Absolute,
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            FocusPolicy::Block,
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.55)),
        ))
        .with_children(|overlay| {
            overlay
                .spawn((
                    Node {
                        width: Val::Px((cfg.slot_px + 6.0) * cfg.cols as f32 + 300.0), // 留出左侧面板宽度
                        height: Val::Px((cfg.slot_px + 6.0) * cfg.rows as f32 + 110.0),
                        flex_direction: FlexDirection::Row,
                        column_gap: Val::Px(12.0),
                        padding: UiRect::all(Val::Px(14.0)),
                        ..default()
                    },
                    BackgroundColor(Color::srgba(0.12, 0.12, 0.16, 0.95)),
                    BorderColor::all(Color::srgb(0.6, 0.6, 0.9)),
                    BorderRadius::all(Val::Px(10.0)),
                ))
                .with_children(|panel| {
                    // 左侧：装备信息面板
                    panel
                        .spawn((
                            Node {
                                width: Val::Px(260.0),
                                height: Val::Percent(100.0),
                                flex_direction: FlexDirection::Column,
                                row_gap: Val::Px(8.0),
                                padding: UiRect::all(Val::Px(8.0)),
                                ..default()
                            },
                            BackgroundColor(Color::srgba(0.10, 0.10, 0.14, 1.0)),
                            BorderRadius::all(Val::Px(6.0)),
                        ))
                        .with_children(|left| {
                            left.spawn((
                                Text::new("已装备".to_string()),
                                TextFont {
                                    font: font.clone(),
                                    font_size: 20.0,
                                    ..default()
                                },
                                TextColor(Color::WHITE),
                            ));

                            // 武器名称
                            let name = equipped.weapon.display_name();
                            left.spawn((
                                Text::new(format!("武器：{}", name)),
                                TextFont {
                                    font: font.clone(),
                                    font_size: 18.0,
                                    ..default()
                                },
                                TextColor(Color::srgb(0.95, 0.95, 0.95)),
                            ));

                            // 类型
                            left.spawn((
                                Text::new(format!("类型：{:?}", equip_set.weapon_kind)),
                                TextFont {
                                    font: font.clone(),
                                    font_size: 16.0,
                                    ..default()
                                },
                                TextColor(Color::srgb(0.85, 0.85, 0.85)),
                            ));

                            // 伤害/冷却
                            left.spawn((
                                Text::new(format!("伤害：{:.1}", equip_set.weapon_damage)),
                                TextFont {
                                    font: font.clone(),
                                    font_size: 16.0,
                                    ..default()
                                },
                                TextColor(Color::srgb(0.9, 0.9, 0.6)),
                            ));
                            left.spawn((
                                Text::new(format!(
                                    "冷却：{:.2}s",
                                    equip_set.weapon_attack_cooldown
                                )),
                                TextFont {
                                    font: font.clone(),
                                    font_size: 16.0,
                                    ..default()
                                },
                                TextColor(Color::srgb(0.9, 0.9, 0.6)),
                            ));

                            // 额外属性：近战范围 or 远程速度
                            match equip_set.weapon_kind {
                                crate::equipment::WeaponKind::Melee => {
                                    left.spawn((
                                        Text::new(format!(
                                            "近战长度：{:.0}",
                                            equip_set.melee_range
                                        )),
                                        TextFont {
                                            font: font.clone(),
                                            font_size: 14.0,
                                            ..default()
                                        },
                                        TextColor(Color::srgb(0.8, 0.8, 0.9)),
                                    ));
                                    left.spawn((
                                        Text::new(format!(
                                            "近战宽度：{:.0}",
                                            equip_set.melee_width
                                        )),
                                        TextFont {
                                            font: font.clone(),
                                            font_size: 14.0,
                                            ..default()
                                        },
                                        TextColor(Color::srgb(0.8, 0.8, 0.9)),
                                    ));
                                }
                                crate::equipment::WeaponKind::Ranged => {
                                    left.spawn((
                                        Text::new(format!(
                                            "弹速：{:.0}",
                                            equip_set.weapon_projectile_speed
                                        )),
                                        TextFont {
                                            font: font.clone(),
                                            font_size: 14.0,
                                            ..default()
                                        },
                                        TextColor(Color::srgb(0.8, 0.8, 0.9)),
                                    ));
                                    left.spawn((
                                        Text::new(format!(
                                            "弹寿命：{:.2}s",
                                            equip_set.weapon_projectile_lifetime
                                        )),
                                        TextFont {
                                            font: font.clone(),
                                            font_size: 14.0,
                                            ..default()
                                        },
                                        TextColor(Color::srgb(0.8, 0.8, 0.9)),
                                    ));
                                }
                            }

                            left.spawn((
                                Text::new("简介：\n这是一件武器，可以装备用于战斗。".to_string()),
                                TextFont {
                                    font: font.clone(),
                                    font_size: 14.0,
                                    ..default()
                                },
                                TextColor(Color::srgb(0.75, 0.75, 0.9)),
                            ));
                        });

                    // 右侧：背包区（Grid + 翻页）
                    panel
                        .spawn((Node {
                            width: Val::Percent(100.0),
                            height: Val::Percent(100.0),
                            flex_direction: FlexDirection::Column,
                            ..default()
                        },))
                        .with_children(|right| {
                            right.spawn((
                                Text::new(format!(
                                    "背包 (B)  Page {}/{}",
                                    page + 1,
                                    page_count.max(1)
                                )),
                                TextFont {
                                    font: font.clone(),
                                    font_size: 22.0,
                                    ..default()
                                },
                                TextColor(Color::WHITE),
                            ));

                            // Grid 容器：Display::Grid + RepeatedGridTrack
                            right
                                .spawn((
                                    Node {
                                        width: Val::Percent(100.0),
                                        height: Val::Px((cfg.slot_px + 6.0) * cfg.rows as f32),
                                        display: Display::Grid,
                                        grid_template_columns: (0..cfg.cols)
                                            .map(|_| RepeatedGridTrack::flex(1, 1.0))
                                            .collect(),
                                        grid_template_rows: (0..cfg.rows)
                                            .map(|_| RepeatedGridTrack::flex(1, 1.0))
                                            .collect(),
                                        row_gap: Val::Px(6.0),
                                        column_gap: Val::Px(6.0),
                                        padding: UiRect::all(Val::Px(10.0)),
                                        ..default()
                                    },
                                    BackgroundColor(Color::srgba(0.08, 0.08, 0.10, 1.0)),
                                    BorderRadius::all(Val::Px(8.0)),
                                ))
                                .with_children(|grid| {
                                    for slot_index in start..end {
                                        let slot = inv.slots[slot_index];
                                        let selected = state.selected == Some(slot_index);

                                        let border = if selected {
                                            Color::srgb(1.0, 0.9, 0.2)
                                        } else {
                                            Color::srgb(0.25, 0.25, 0.35)
                                        };

                                        grid.spawn((
                                            Button,
                                            SlotButton { slot_index },
                                            Node {
                                                width: Val::Px(cfg.slot_px),
                                                height: Val::Px(cfg.slot_px),
                                                justify_content: JustifyContent::Center,
                                                align_items: AlignItems::Center,
                                                padding: UiRect::all(Val::Px(2.0)),
                                                ..default()
                                            },
                                            BackgroundColor(Color::srgb(0.18, 0.18, 0.24)),
                                            BorderColor::all(border),
                                            BorderRadius::all(Val::Px(6.0)),
                                        ))
                                        .with_children(
                                            |cell| {
                                                if let Some(ItemStack { id, count }) = slot {
                                                    // Load icon via asset_server; 这里要求你在 equipment::ItemId 提供 icon_path()
                                                    let icon_path = id.icon_path();
                                                    let icon_handle: Handle<Image> =
                                                        asset_server.load(icon_path);

                                                    // Bevy 0.17.3: ImageNode::new(handle) 是显示图片的方式
                                                    cell.spawn((
                                                        ImageNode::new(icon_handle),
                                                        Node {
                                                            width: Val::Percent(100.0),
                                                            height: Val::Percent(100.0),
                                                            ..default()
                                                        },
                                                    ));

                                                    // 右下角数量
                                                    cell.spawn((
                                                        Node {
                                                            position_type: PositionType::Absolute,
                                                            right: Val::Px(4.0),
                                                            bottom: Val::Px(2.0),
                                                            ..default()
                                                        },
                                                        Text::new(format!("{}", count)),
                                                        TextFont {
                                                            font: font.clone(),
                                                            font_size: 14.0,
                                                            ..default()
                                                        },
                                                        TextColor(Color::WHITE),
                                                    ));
                                                }
                                            },
                                        );
                                    }
                                });

                            // 翻页栏
                            right
                                .spawn((Node {
                                    width: Val::Percent(100.0),
                                    height: Val::Px(40.0),
                                    flex_direction: FlexDirection::Row,
                                    justify_content: JustifyContent::SpaceBetween,
                                    align_items: AlignItems::Center,
                                    ..default()
                                },))
                                .with_children(|bar| {
                                    bar.spawn((
                                        Button,
                                        PrevPageBtn,
                                        Node {
                                            width: Val::Px(90.0),
                                            height: Val::Px(32.0),
                                            ..default()
                                        },
                                        BackgroundColor(Color::srgb(0.25, 0.25, 0.35)),
                                        BorderRadius::all(Val::Px(6.0)),
                                    ))
                                    .with_children(|b| {
                                        b.spawn((
                                            Text::new("< Prev"),
                                            TextFont {
                                                font: font.clone(),
                                                font_size: 16.0,
                                                ..default()
                                            },
                                            TextColor(Color::WHITE),
                                        ));
                                    });

                                    bar.spawn((
                                        Text::new("点击格子可装备/交换（仅武器槽）"),
                                        TextFont {
                                            font: font.clone(),
                                            font_size: 14.0,
                                            ..default()
                                        },
                                        TextColor(Color::srgb(0.75, 0.75, 0.9)),
                                    ));

                                    bar.spawn((
                                        Button,
                                        NextPageBtn,
                                        Node {
                                            width: Val::Px(90.0),
                                            height: Val::Px(32.0),
                                            ..default()
                                        },
                                        BackgroundColor(Color::srgb(0.25, 0.25, 0.35)),
                                        BorderRadius::all(Val::Px(6.0)),
                                    ))
                                    .with_children(|b| {
                                        b.spawn((
                                            Text::new("Next >"),
                                            TextFont {
                                                font: font.clone(),
                                                font_size: 16.0,
                                                ..default()
                                            },
                                            TextColor(Color::WHITE),
                                        ));
                                    });
                                });
                        });
                });
        });
}

fn handle_inventory_ui_interactions(
    mut slot_q: Query<(&Interaction, &SlotButton), (Changed<Interaction>, With<Button>)>,
    mut prev_q: Query<&Interaction, (Changed<Interaction>, With<PrevPageBtn>)>,
    mut next_q: Query<&Interaction, (Changed<Interaction>, With<NextPageBtn>)>,
    mut slot_writer: MessageWriter<InventorySlotClickMsg>,
    mut page_writer: MessageWriter<InventoryPageMsg>,
) {
    for (it, btn) in &mut slot_q {
        if *it == Interaction::Pressed {
            slot_writer.write(InventorySlotClickMsg {
                slot_index: btn.slot_index,
            });
        }
    }
    if let Ok(it) = prev_q.single_mut() {
        if *it == Interaction::Pressed {
            page_writer.write(InventoryPageMsg { delta: -1 });
        }
    }
    if let Ok(it) = next_q.single_mut() {
        if *it == Interaction::Pressed {
            page_writer.write(InventoryPageMsg { delta: 1 });
        }
    }
}

/// 这里是关键：当读取到背包格子点击消息时，执行“装备/交换”逻辑
fn apply_inventory_ui_messages(
    cfg: Res<InventoryUiConfig>,
    mut state: ResMut<InventoryUiState>,
    mut inv_q: Query<&mut Inventory, With<Player>>,
    mut equip_q: Query<(&mut EquippedItems, &mut EquipmentSet), With<Player>>,
    db: Res<ItemDatabase>,
    mut slot_reader: MessageReader<InventorySlotClickMsg>,
    mut page_reader: MessageReader<InventoryPageMsg>,
) {
    let mut inv = match inv_q.single_mut() {
        Ok(i) => i,
        Err(_) => return,
    };

    let mut equip_pair = match equip_q.single_mut() {
        Ok(p) => p,
        Err(_) => return,
    };

    let page_size = cfg.cols * cfg.rows;
    let page_count = (inv.slot_count() + page_size - 1) / page_size;

    // 处理格子点击 — 装备逻辑
    for m in slot_reader.read() {
        let idx = m.slot_index;
        if idx >= inv.slot_count() {
            continue;
        }

        // 选中但空格不做交换
        if inv.slots[idx].is_none() {
            state.selected = Some(idx);
            continue;
        }

        // 有物品：尝试作为武器装备
        let mut stack = inv.slots[idx].unwrap();
        let old_weapon = equip_pair.0.weapon;

        // 如果点的是已装备的同名武器 —— 直接不操作
        if stack.id == old_weapon {
            state.selected = Some(idx);
            continue;
        }

        // 从格子中“消耗”一把来装备（如果堆叠>1 则减1，否则清空该格）
        if stack.count > 1 {
            stack.count -= 1;
            inv.slots[idx] = Some(stack);
        } else {
            inv.slots[idx] = None;
        }

        // 把旧武器放回背包（叠加逻辑）
        let remaining = inv.try_add(old_weapon, 1, 99);
        if remaining > 0 {
            // 背包放不下：尝试放回当前格（如果现在空）
            if inv.slots[idx].is_none() {
                inv.slots[idx] = Some(ItemStack {
                    id: old_weapon,
                    count: 1,
                });
            } else {
                // 放不下也无法回包：丢弃（或提示）
            }
        }

        // 装上新武器（更新装备与战斗参数）
        equip_pair.0.weapon = stack.id;
        if let Some(def) = db.weapon(stack.id) {
            *equip_pair.1 = EquipmentSet::from_weapon(def);
        }

        state.selected = Some(idx);
    }

    // 处理翻页
    for m in page_reader.read() {
        let mut p = state.page as i32 + m.delta;
        if page_count == 0 {
            p = 0;
        }
        p = p.clamp(0, (page_count.saturating_sub(1)) as i32);
        state.page = p as usize;
    }
}
