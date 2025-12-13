// src/inventory_ui.rs
use bevy::prelude::*;
use bevy::ui::{RepeatedGridTrack, Display, BorderRadius, BorderColor};

use crate::equipment::ItemId;
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
            toggle_key: KeyCode::KeyI,
            cols: 10,
            rows: 4,     // 每页 40 格
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
struct SlotButton { slot_index: usize }

#[derive(Component)]
struct PrevPageBtn;
#[derive(Component)]
struct NextPageBtn;

#[derive(Message, Clone, Copy, Debug)]
pub struct InventorySlotClickMsg { pub slot_index: usize }

#[derive(Message, Clone, Copy, Debug)]
pub struct InventoryPageMsg { pub delta: i32 }

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

/// 只要状态有变就重建（先做简单策略）
fn rebuild_inventory_ui_on_change(
    mut commands: Commands,
    cfg: Res<InventoryUiConfig>,
    state: Res<InventoryUiState>,
    q_root: Query<Entity, With<InventoryUiRoot>>,
    q_player: Query<&Inventory, With<Player>>,
    asset_server: Res<AssetServer>,
) {
    // 清旧 UI（如果有）
    if let Ok(root) = q_root.single() {
        commands.entity(root).try_despawn();
    }

    if !state.open {
        return;
    }

    let Ok(inv) = q_player.single() else {
        return;
    };

    spawn_inventory_ui(&mut commands, &asset_server, &cfg, &*state, inv);
}

fn spawn_inventory_ui(
    commands: &mut Commands,
    asset_server: &AssetServer,
    cfg: &InventoryUiConfig,
    state: &InventoryUiState,
    inv: &Inventory,
) {
    let font: Handle<Font> = asset_server.load("fonts/YuFanLixing.otf");

    let page_size = cfg.cols * cfg.rows;
    let page_count = (inv.slot_count() + page_size - 1) / page_size;
    let page = state.page.min(page_count.saturating_sub(1));
    let start = page * page_size;
    let end = (start + page_size).min(inv.slot_count());

    commands.spawn((
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
        overlay.spawn((
            Node {
                width: Val::Px((cfg.slot_px + 6.0) * cfg.cols as f32 + 40.0),
                height: Val::Px((cfg.slot_px + 6.0) * cfg.rows as f32 + 110.0),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(10.0),
                padding: UiRect::all(Val::Px(14.0)),
                ..default()
            },
            BackgroundColor(Color::srgba(0.12, 0.12, 0.16, 0.95)),
            BorderColor::all(Color::srgb(0.6, 0.6, 0.9)),
            BorderRadius::all(Val::Px(10.0)),
        ))
        .with_children(|panel| {
            // 标题
            panel.spawn((
                Text::new(format!("背包 (I)  Page {}/{}", page + 1, page_count.max(1))),
                TextFont { font: font.clone(), font_size: 22.0, ..default() },
                TextColor(Color::WHITE),
            ));

            // Grid 容器：Display::Grid + RepeatedGridTrack
            panel.spawn((
                Node {
                    width: Val::Percent(100.0),
                    height: Val::Px((cfg.slot_px + 6.0) * cfg.rows as f32),
                    display: Display::Grid,
                    grid_template_columns: (0..cfg.cols).map(|_| RepeatedGridTrack::flex(1, 1.0)).collect(),
                    grid_template_rows: (0..cfg.rows).map(|_| RepeatedGridTrack::flex(1, 1.0)).collect(),
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

                    let border = if selected { Color::srgb(1.0, 0.9, 0.2) } else { Color::srgb(0.25, 0.25, 0.35) };

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
                    .with_children(|cell| {
                        if let Some(ItemStack { id, count }) = slot {
                            // Load icon via asset_server; 这里要求你在 equipment::ItemId 提供 icon_path()
                            let icon_path = id.icon_path();
                            let icon_handle: Handle<Image> = asset_server.load(icon_path);

                            // ImageBundle in bevy 0.17 是 ImageBundle { image: UiImage(handle), style: Style{...}, ..default() }
                            cell.spawn((
                                ImageBundle {
                                    image: UiImage(icon_handle),
                                    style: Style {
                                        size: Size::new(Val::Percent(100.0), Val::Percent(100.0)),
                                        ..default()
                                    },
                                    ..default()
                                },
                            ));

                            // 右下角数量 (使用你项目中已有的 Text/Font wrappers)
                            cell.spawn((
                                Node {
                                    position_type: PositionType::Absolute,
                                    right: Val::Px(4.0),
                                    bottom: Val::Px(2.0),
                                    ..default()
                                },
                                Text::new(format!("{}", count)),
                                TextFont { font: font.clone(), font_size: 14.0, ..default() },
                                TextColor(Color::WHITE),
                            ));
                        }
                    });
                }
            });

            // 翻页栏
            panel.spawn((
                Node {
                    width: Val::Percent(100.0),
                    height: Val::Px(40.0),
                    flex_direction: FlexDirection::Row,
                    justify_content: JustifyContent::SpaceBetween,
                    align_items: AlignItems::Center,
                    ..default()
                },
            ))
            .with_children(|bar| {
                bar.spawn((Button, PrevPageBtn, Node { width: Val::Px(90.0), height: Val::Px(32.0), ..default() },
                    BackgroundColor(Color::srgb(0.25, 0.25, 0.35)), BorderRadius::all(Val::Px(6.0))))
                    .with_children(|b| {
                        b.spawn((Text::new("< Prev"), TextFont { font: font.clone(), font_size: 16.0, ..default() }, TextColor(Color::WHITE)));
                    });

                bar.spawn((Text::new("点击格子选择物品（后续可：双击装备 / 拖拽交换）"),
                    TextFont { font: font.clone(), font_size: 14.0, ..default() }, TextColor(Color::srgb(0.75, 0.75, 0.9))));

                bar.spawn((Button, NextPageBtn, Node { width: Val::Px(90.0), height: Val::Px(32.0), ..default() },
                    BackgroundColor(Color::srgb(0.25, 0.25, 0.35)), BorderRadius::all(Val::Px(6.0))))
                    .with_children(|b| {
                        b.spawn((Text::new("Next >"), TextFont { font: font.clone(), font_size: 16.0, ..default() }, TextColor(Color::WHITE)));
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
            slot_writer.write(InventorySlotClickMsg { slot_index: btn.slot_index });
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

fn apply_inventory_ui_messages(
    cfg: Res<InventoryUiConfig>,
    mut state: ResMut<InventoryUiState>,
    inv_q: Query<&Inventory, With<Player>>,
    mut slot_reader: MessageReader<InventorySlotClickMsg>,
    mut page_reader: MessageReader<InventoryPageMsg>,
) {
    let Ok(inv) = inv_q.single() else { return; };
    let page_size = cfg.cols * cfg.rows;
    let page_count = (inv.slot_count() + page_size - 1) / page_size;

    for m in slot_reader.read() {
        state.selected = Some(m.slot_index);
    }
    for m in page_reader.read() {
        let mut p = state.page as i32 + m.delta;
        if page_count == 0 { p = 0; }
        p = p.clamp(0, (page_count.saturating_sub(1)) as i32);
        state.page = p as usize;
    }
}
