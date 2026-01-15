use bevy::prelude::*;
use bevy::ui::Val;

use crate::save::{LoadSlotEvent, ManualSaveEvent, SaveSlots};
use crate::ui::types::SelectedSlot;
use crate::utils::despawn_with_children;

#[derive(Component)]
pub struct SavePanel;

#[derive(Component)]
pub struct SavePanelOverlay;

#[derive(Component)]
pub struct SaveSlotsList;

#[derive(Component)]
pub struct ActivateButton;

#[derive(Component)]
pub struct SaveSlotButton {
    pub file_name: String,
    pub action: SaveSlotAction,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum SaveSlotAction {
    Save,
    Select,
}

const REFRESH_SECS: f32 = 0.5;

pub fn open_save_panel(commands: &mut Commands, asset_server: &AssetServer) {
    let font = asset_server.load("fonts/YuFanLixing.otf");

    commands
        .spawn((
            SavePanelOverlay,
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                position_type: PositionType::Absolute,
                left: Val::Px(0.0),
                top: Val::Px(0.0),
                ..default()
            },
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.55)),
        ))
        .with_children(|root| {
            root.spawn((
                SavePanel,
                Node {
                    width: Val::Percent(90.0),
                    max_width: Val::Px(760.0),
                    height: Val::Percent(80.0),
                    max_height: Val::Px(560.0),
                    padding: UiRect::all(Val::Px(16.0)),
                    row_gap: Val::Px(12.0),
                    flex_direction: FlexDirection::Column,
                    justify_content: JustifyContent::FlexStart,
                    align_items: AlignItems::Stretch,
                    ..default()
                },
                BackgroundColor(Color::srgba(0.12, 0.12, 0.16, 0.96)),
            ))
            .with_children(|panel| {
                panel.spawn((
                    Text::new("存档"),
                    TextFont {
                        font: font.clone(),
                        font_size: 30.0,
                        ..default()
                    },
                    TextColor(Color::WHITE),
                ));

                panel.spawn((
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

                panel
                    .spawn(Node {
                        width: Val::Percent(100.0),
                        height: Val::Auto,
                        justify_content: JustifyContent::SpaceBetween,
                        align_items: AlignItems::Center,
                        column_gap: Val::Px(12.0),
                        ..default()
                    })
                    .with_children(|bar| {
                        spawn_action_button(
                            bar,
                            &font,
                            "手动保存",
                            Color::srgb(0.45, 0.35, 0.85),
                            SaveSlotButton {
                                file_name: String::new(),
                                action: SaveSlotAction::Save,
                            },
                        );

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
                                TextFont {
                                    font: font.clone(),
                                    font_size: 20.0,
                                    ..default()
                                },
                                TextColor(Color::WHITE),
                            ));
                        });
                    });
            });
        });
}

pub fn sync_save_slots_list(
    mut commands: Commands,
    time: Res<Time>,
    mut refresh_timer: Local<Option<Timer>>,
    panels_added: Query<Entity, Added<SavePanel>>,
    list_q: Query<Entity, With<SaveSlotsList>>,
    children_q: Query<&Children>,
    asset_server: Res<AssetServer>,
    mut slots: ResMut<SaveSlots>,
    selected: Res<SelectedSlot>,
) {
    let Some(list_e) = list_q.iter().next() else {
        return;
    };

    let timer = refresh_timer
        .get_or_insert_with(|| Timer::from_seconds(REFRESH_SECS, TimerMode::Repeating));
    timer.tick(time.delta());

    let just_opened = !panels_added.is_empty();
    let should_refresh_disk = just_opened || timer.just_finished();

    if should_refresh_disk {
        crate::save::refresh_save_slots_from_disk(&mut slots);
    }

    if !(just_opened || should_refresh_disk || slots.is_changed() || selected.is_changed()) {
        return;
    }

    if let Ok(children) = children_q.get(list_e) {
        let old_children: Vec<Entity> = children.iter().collect();
        for e in old_children {
            despawn_with_children(&mut commands, &children_q, e);
        }
    }

    let font = asset_server.load("fonts/YuFanLixing.otf");
    let cur = selected.0.clone();

    commands.entity(list_e).with_children(|parent| {
        if slots.slots.is_empty() {
            parent.spawn((
                Text::new("暂无存档（请先手动保存一次）"),
                TextFont {
                    font: font.clone(),
                    font_size: 18.0,
                    ..default()
                },
                TextColor(Color::srgb(0.8, 0.8, 0.8)),
            ));
            return;
        }

        for meta in slots.slots.iter() {
            let is_selected = cur.as_deref() == Some(meta.file_name.as_str());
            let label = if meta.is_auto {
                format!("{}  (自动)", meta.display_name)
            } else {
                meta.display_name.clone()
            };

            parent
                .spawn((
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
                        TextFont {
                            font: font.clone(),
                            font_size: 18.0,
                            ..default()
                        },
                        TextColor(Color::WHITE),
                    ));
                });
        }
    });
}

pub fn handle_save_slot_buttons(
    mut interactions: Query<
        (&Interaction, &mut BackgroundColor, &SaveSlotButton),
        Changed<Interaction>,
    >,
    mut manual_save_tx: MessageWriter<ManualSaveEvent>,
    mut selected_slot: ResMut<SelectedSlot>,
) {
    for (interaction, mut bg, btn) in interactions.iter_mut() {
        let base = match btn.action {
            SaveSlotAction::Save => Color::srgb(0.45, 0.35, 0.85),
            SaveSlotAction::Select => Color::srgb(0.20, 0.20, 0.26),
        };

        match *interaction {
            Interaction::Pressed => {
                bg.0 = Color::srgb(0.8, 0.8, 1.0);
                match btn.action {
                    SaveSlotAction::Save => {
                        manual_save_tx.write(ManualSaveEvent {
                            file_name: None,
                            slot_index: None,
                        });
                    }
                    SaveSlotAction::Select => {
                        selected_slot.0 = Some(btn.file_name.clone());
                    }
                }
            }
            Interaction::Hovered => bg.0 = Color::srgb(0.6, 0.6, 0.8),
            Interaction::None => bg.0 = base,
        }
    }
}

pub fn handle_activate_button(
    mut interactions: Query<
        (&Interaction, &mut BackgroundColor),
        (Changed<Interaction>, With<ActivateButton>),
    >,
    selected_slot: Res<SelectedSlot>,
    mut load_tx: MessageWriter<LoadSlotEvent>,
    mut commands: Commands,
    q_overlay: Query<Entity, With<SavePanelOverlay>>,
    children_q: Query<&Children>,
) {
    for (interaction, mut bg) in interactions.iter_mut() {
        match *interaction {
            Interaction::Pressed => {
                bg.0 = Color::srgb(0.8, 0.8, 1.0);

                if let Some(name) = selected_slot.0.clone() {
                    load_tx.write(LoadSlotEvent { file_name: name });
                }

                if let Some(root) = q_overlay.iter().next() {
                    despawn_with_children(&mut commands, &children_q, root);
                }
            }
            Interaction::Hovered => bg.0 = Color::srgb(0.35, 0.75, 0.50),
            Interaction::None => bg.0 = Color::srgb(0.25, 0.55, 0.35),
        }
    }
}

pub fn close_save_panel_on_esc(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut commands: Commands,
    q_overlay: Query<Entity, With<SavePanelOverlay>>,
    children_q: Query<&Children>,
) {
    if !keyboard.just_pressed(KeyCode::Escape) {
        return;
    }

    if let Some(root) = q_overlay.iter().next() {
        despawn_with_children(&mut commands, &children_q, root);
    }
}

fn spawn_action_button(
    parent: &mut ChildSpawnerCommands<'_>,
    font: &Handle<Font>,
    label: &str,
    color: Color,
    button: SaveSlotButton,
) {
    parent
        .spawn((
            Button,
            Node {
                width: Val::Px(180.0),
                height: Val::Px(44.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(color),
            button,
        ))
        .with_children(|btn| {
            btn.spawn((
                Text::new(label),
                TextFont {
                    font: font.clone(),
                    font_size: 20.0,
                    ..default()
                },
                TextColor(Color::WHITE),
            ));
        });
}
