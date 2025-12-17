use bevy::prelude::*;
use bevy::ui::Val;

use crate::save::{LoadSlotEvent, ManualSaveEvent};
use crate::ui::types::SelectedSlot;

#[derive(Component)]
pub struct SavePanel;

#[derive(Component)]
pub struct ActivateButton;

#[derive(Component)]
pub struct SaveSlotButton {
    pub file_name: String,
    pub action: SaveSlotAction,
}

#[derive(Clone, Copy)]
pub enum SaveSlotAction {
    Save,
    Select,
}

pub fn open_save_panel(commands: &mut Commands) {
    spawn_save_panel(commands);
}

fn spawn_save_panel(commands: &mut Commands) {
    commands
        .spawn((
            SavePanel,
            Node {
                width: Val::Px(640.0),
                height: Val::Px(420.0),
                position_type: PositionType::Absolute,
                left: Val::Percent(50.0),
                top: Val::Percent(50.0),
                margin: UiRect::all(Val::Px(0.0)),
                ..default()
            },
            Transform::from_translation(Vec3::new(-320.0, -210.0, 200.0)),
            BackgroundColor(Color::srgba(0.12, 0.12, 0.16, 0.95)),
        ))
        .with_children(|parent| {
            // 标题
            parent.spawn((
                Text::new("存档面板"),
                TextFont {
                    font_size: 28.0,
                    ..default()
                },
                TextColor(Color::WHITE),
            ));

            parent
                .spawn((
                    Button,
                    Node {
                        width: Val::Px(220.0),
                        height: Val::Px(44.0),
                        margin: UiRect::all(Val::Px(8.0)),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        ..default()
                    },
                    BackgroundColor(Color::srgb(0.45, 0.35, 0.85)),
                    SaveSlotButton {
                        file_name: "autosave.json".to_string(),
                        action: SaveSlotAction::Save,
                    },
                ))
                .with_children(|btn| {
                    btn.spawn((
                        Text::new("手动保存"),
                        TextFont {
                            font_size: 20.0,
                            ..default()
                        },
                        TextColor(Color::WHITE),
                    ));
                });

            parent
                .spawn((
                    Button,
                    Node {
                        width: Val::Px(220.0),
                        height: Val::Px(44.0),
                        margin: UiRect::all(Val::Px(8.0)),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        ..default()
                    },
                    BackgroundColor(Color::srgb(0.25, 0.55, 0.35)),
                    ActivateButton,
                ))
                .with_children(|btn| {
                    btn.spawn((
                        Text::new("载入已选择存档"),
                        TextFont {
                            font_size: 20.0,
                            ..default()
                        },
                        TextColor(Color::WHITE),
                    ));
                });
        });
}

pub fn sync_save_slots_list() {}

pub fn handle_save_panel_actions(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut commands: Commands,
    q: Query<Entity, With<SavePanel>>,
) {
    if keyboard.just_pressed(KeyCode::Escape) {
        if let Ok(panel) = q.single() {
            commands.entity(panel).try_despawn();
        }
    }
}

pub fn close_save_panel_on_esc(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut commands: Commands,
    q: Query<Entity, With<SavePanel>>,
) {
    if keyboard.just_pressed(KeyCode::Escape) {
        if let Ok(panel) = q.single() {
            commands.entity(panel).try_despawn();
        }
    }
}

pub fn handle_save_slot_buttons(
    mut interactions: Query<(&Interaction, &mut BackgroundColor, Option<&SaveSlotButton>), Changed<Interaction>>,
    mut load_tx: MessageWriter<LoadSlotEvent>,
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
                            manual_save_tx.write(ManualSaveEvent {
                                file_name: None,
                                slot_index: None,
                            });
                        }
                        SaveSlotAction::Select => {
                            selected_slot.0 = Some(btn.file_name.clone());
                            load_tx.write(LoadSlotEvent {
                                file_name: btn.file_name.clone(),
                            });
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
