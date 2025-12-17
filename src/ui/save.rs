use bevy::prelude::*;
use bevy::ui::Val;

use crate::save::{LoadSlotEvent, ManualSaveEvent, SaveSlots};
use crate::ui::types::SelectedSlot;

#[derive(Component)]
pub struct SavePanel;

#[derive(Component)]
pub struct SavePanelOverlay;

#[derive(Component)]
pub struct SaveSlotsList;

#[derive(Component)]
pub struct SlotRow;

#[derive(Component)]
pub struct ActivateButton;

#[derive(Component)]
pub struct SaveSlotButton {
    pub file_name: String,
    pub action: SaveSlotAction,
}

#[derive(Clone, Copy)]
pub enum SaveSlotAction {
    Save,   // 手动保存：创建新存档
    Select, // 选择某个存档（不直接加载）
}

pub fn open_save_panel(commands: &mut Commands) {
    // 防止重复打开
    commands.spawn((SavePanelOverlay, Node {
        width: Val::Percent(100.0),
        height: Val::Percent(100.0),
        justify_content: JustifyContent::Center,
        align_items: AlignItems::Center,
        position_type: PositionType::Absolute,
        left: Val::Px(0.0),
        top: Val::Px(0.0),
        ..default()
    }, BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.55))));

    // 真正面板
    commands
        .spawn((
            SavePanel,
            Node {
                width: Val::Percent(80.0),
                // 这里不写死像素：尽量自适应；如果你想更稳，可以再加 max_width / max_height（看你项目 Node 字段是否启用）
                height: Val::Percent(75.0),
                padding: UiRect::all(Val::Px(16.0)),
                row_gap: Val::Px(12.0),
                flex_direction: FlexDirection::Column,
                justify_content: JustifyContent::FlexStart,
                align_items: AlignItems::Stretch,
                ..default()
            },
            BackgroundColor(Color::srgba(0.12, 0.12, 0.16, 0.96)),
        ))
        .with_children(|parent| {
            // 标题
            parent.spawn((
                Text::new("存档"),
                TextFont { font_size: 30.0, ..default() },
                TextColor(Color::WHITE),
            ));

            // 存档列表（可滚动）
            parent.spawn((
                SaveSlotsList,
                Node {
                    width: Val::Percent(100.0),
                    height: Val::Percent(65.0),
                    padding: UiRect::all(Val::Px(8.0)),
                    row_gap: Val::Px(6.0),
                    flex_direction: FlexDirection::Column,
                    overflow: Overflow::scroll_y(),
                    ..default()
                },
                BackgroundColor(Color::srgba(0.08, 0.08, 0.10, 0.9)),
            ));

            // 底部按钮区
            parent.spawn((
                Node {
                    width: Val::Percent(100.0),
                    height: Val::Auto,
                    justify_content: JustifyContent::SpaceBetween,
                    align_items: AlignItems::Center,
                    column_gap: Val::Px(12.0),
                    ..default()
                },
            ))
            .with_children(|bar| {
                // 手动保存（新建）
                bar.spawn((
                    Button,
                    Node {
                        width: Val::Px(180.0),
                        height: Val::Px(44.0),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        ..default()
                    },
                    BackgroundColor(Color::srgb(0.45, 0.35, 0.85)),
                    SaveSlotButton { file_name: String::new(), action: SaveSlotAction::Save },
                ))
                .with_children(|btn| {
                    btn.spawn((
                        Text::new("手动保存"),
                        TextFont { font_size: 20.0, ..default() },
                        TextColor(Color::WHITE),
                    ));
                });

                // 激活（加载选中存档）
                bar.spawn((
                    Button,
                    Node {
                        width: Val::Px(220.0),
                        height: Val::Px(44.0),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        ..default()
                    },
                    BackgroundColor(Color::srgb(0.25, 0.55, 0.35)),
                    ActivateButton,
                ))
                .with_children(|btn| {
                    btn.spawn((
                        Text::new("载入选中存档"),
                        TextFont { font_size: 20.0, ..default() },
                        TextColor(Color::WHITE),
                    ));
                });
            });
        });
}

/// 关键：
/// - 面板刚打开时扫描磁盘（Paused/InGame 打开也能刷新）
/// - SaveSlots / SelectedSlot 变化时重建列表
pub fn sync_save_slots_list(
    mut commands: Commands,
    panels_added: Query<Entity, Added<SavePanel>>,
    list_q: Query<Entity, With<SaveSlotsList>>,
    mut slots: ResMut<SaveSlots>,
    selected: Res<SelectedSlot>,
) {
    let just_opened = !panels_added.is_empty();
    if just_opened {
        // 扫描 ./saves
        crate::save::refresh_save_slots_from_disk(&mut slots);
    }

    if !(just_opened || slots.is_changed() || selected.is_changed()) {
        return;
    }

    let Ok(list_e) = list_q.single() else { return; };

    // 清空旧列表
    commands.entity(list_e).despawn();

    // 重新生成列表
    let cur = selected.0.clone();
    commands.entity(list_e).with_children(|parent| {
        for meta in &slots.slots {
            let is_selected = cur.as_deref() == Some(meta.file_name.as_str());

            let label = if meta.is_auto {
                format!("{}  (自动)", meta.display_name)
            } else {
                meta.display_name.clone()
            };

            parent.spawn((
                Button,
                Node {
                    width: Val::Percent(100.0),
                    height: Val::Px(40.0),
                    padding: UiRect::horizontal(Val::Px(10.0)),
                    justify_content: JustifyContent::FlexStart,
                    align_items: AlignItems::Center,
                    ..default()
                },
                BackgroundColor(if is_selected {
                    Color::srgb(0.35, 0.40, 0.55)
                } else {
                    Color::srgb(0.20, 0.20, 0.26)
                }),
                SaveSlotButton {
                    file_name: meta.file_name.clone(),
                    action: SaveSlotAction::Select,
                },
            ))
            .with_children(|row| {
                row.spawn((
                    Text::new(label),
                    TextFont { font_size: 18.0, ..default() },
                    TextColor(Color::WHITE),
                ));
            });
        }

        if slots.slots.is_empty() {
            parent.spawn((
                Text::new("暂无存档（请先手动保存一次）"),
                TextFont { font_size: 18.0, ..default() },
                TextColor(Color::srgb(0.8, 0.8, 0.8)),
            ));
        }
    });
}

pub fn handle_save_slot_buttons(
    mut interactions: Query<(&Interaction, &mut BackgroundColor, Option<&SaveSlotButton>), Changed<Interaction>>,
    mut manual_save_tx: MessageWriter<ManualSaveEvent>,
    mut selected_slot: ResMut<SelectedSlot>,
) {
    for (interaction, mut bg, btn) in &mut interactions {
        match *interaction {
            Interaction::Pressed => {
                bg.0 = Color::srgb(0.8, 0.8, 1.0);

                if let Some(btn) = btn {
                    match btn.action {
                        SaveSlotAction::Save => {
                            // 新建当天序号存档
                            manual_save_tx.write(ManualSaveEvent { file_name: None, slot_index: None });
                        }
                        SaveSlotAction::Select => {
                            // 只选择，不加载
                            selected_slot.0 = Some(btn.file_name.clone());
                        }
                    }
                }
            }
            Interaction::Hovered => bg.0 = Color::srgb(0.6, 0.6, 0.8),
            Interaction::None => bg.0 = Color::srgb(0.25, 0.25, 0.35),
        }
    }
}

pub fn handle_activate_button(
    mut interactions: Query<(&Interaction, &mut BackgroundColor), (Changed<Interaction>, With<ActivateButton>)>,
    selected_slot: Res<SelectedSlot>,
    mut load_tx: MessageWriter<LoadSlotEvent>,
) {
    for (interaction, mut bg) in &mut interactions {
        match *interaction {
            Interaction::Pressed => {
                bg.0 = Color::srgb(0.8, 0.8, 1.0);
                if let Some(name) = selected_slot.0.clone() {
                    load_tx.write(LoadSlotEvent { file_name: name });
                }
            }
            Interaction::Hovered => bg.0 = Color::srgb(0.6, 0.6, 0.8),
            Interaction::None => bg.0 = Color::srgb(0.25, 0.25, 0.35),
        }
    }
}

// 关闭时把 overlay 和 panel 一起关掉（你也可以用一个更统一的 Root 包起来）
pub fn close_save_panel_on_esc(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut commands: Commands,
    q_panel: Query<Entity, With<SavePanel>>,
    q_overlay: Query<Entity, With<SavePanelOverlay>>,
) {
    if !keyboard.just_pressed(KeyCode::Escape) {
        return;
    }
    if let Ok(e) = q_panel.single() {
        commands.entity(e).try_despawn();
    }
    if let Ok(e) = q_overlay.single() {
        commands.entity(e).try_despawn();
    }
}