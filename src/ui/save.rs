use bevy::prelude::*;
use bevy::ui::Val;

use crate::i18n::{L10n, Language};
use crate::save::{DeleteSlotEvent, LoadSlotEvent, ManualSaveEvent, SaveSlots};
use crate::ui::EscBlockingUi;
use crate::ui::skin;
use crate::ui::types::{GameSettings, SelectedSlot};
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
pub struct DeleteButton;

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

pub fn open_save_panel(commands: &mut Commands, asset_server: &AssetServer, lang: Language) {
    let font = skin::ui_font(asset_server, lang);

    commands
        .spawn((
            SavePanelOverlay,
            EscBlockingUi,
            GlobalZIndex(300),
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
            BackgroundColor(skin::overlay()),
        ))
        .with_children(|root| {
            root.spawn((
                SavePanel,
                Node {
                    width: Val::Percent(90.0),
                    max_width: Val::Px(760.0),
                    height: Val::Percent(80.0),
                    max_height: Val::Px(560.0),
                    padding: UiRect::all(Val::Px(20.0)),
                    row_gap: Val::Px(14.0),
                    flex_direction: FlexDirection::Column,
                    justify_content: JustifyContent::FlexStart,
                    align_items: AlignItems::Stretch,
                    border: UiRect::all(Val::Px(1.5)),
                    border_radius: skin::radius(skin::PANEL_RADIUS),
                    ..default()
                },
                skin::panel_decoration(),
                UiTransform::IDENTITY,
                crate::ui::anim::UiTween::pop_in(0.28, 0.95),
            ))
            .with_children(|panel| {
                panel.spawn((
                    Text::new(L10n::save_title(lang)),
                    TextFont {
                        font: font.clone().into(),
                        font_size: FontSize::from(skin::FONT_HEADING + 4.0),
                        ..default()
                    },
                    TextColor(skin::text_accent()),
                ));

                panel.spawn((
                    SaveSlotsList,
                    Node {
                        width: Val::Percent(100.0),
                        height: Val::Percent(65.0),
                        padding: UiRect::all(Val::Px(10.0)),
                        row_gap: Val::Px(8.0),
                        flex_direction: FlexDirection::Column,
                        overflow: Overflow::scroll_y(),
                        border_radius: skin::radius(10.0),
                        ..default()
                    },
                    BackgroundColor(skin::inset_tint()),
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
                            asset_server,
                            &font,
                            L10n::save_manual(lang),
                            skin::button_primary(),
                            SaveSlotButton {
                                file_name: String::new(),
                                action: SaveSlotAction::Save,
                            },
                        );

                        bar.spawn((
                            Button,
                            Node {
                                width: Val::Px(220.0),
                                height: Val::Px(46.0),
                                justify_content: JustifyContent::Center,
                                align_items: AlignItems::Center,
                                border: UiRect::all(Val::Px(1.5)),
                                border_radius: skin::radius(skin::BUTTON_RADIUS),
                                ..default()
                            },
                            BackgroundColor(skin::button_confirm()),
                            BorderColor::all(skin::border_soft()),
                            skin::shadow_card(),
                            UiTransform::IDENTITY,
                            crate::ui::anim::HoverMotion::default(),
                            ActivateButton,
                        ))
                        .with_children(|btn| {
                            btn.spawn((
                                Text::new(L10n::save_load_selected(lang)),
                                TextFont {
                                    font: font.clone().into(),
                                    font_size: FontSize::from(skin::FONT_BODY + 2.0),
                                    ..default()
                                },
                                TextColor(skin::text_primary()),
                            ));
                        });

                        bar.spawn((
                            Button,
                            Node {
                                width: Val::Px(220.0),
                                height: Val::Px(46.0),
                                justify_content: JustifyContent::Center,
                                align_items: AlignItems::Center,
                                border: UiRect::all(Val::Px(1.5)),
                                border_radius: skin::radius(skin::BUTTON_RADIUS),
                                ..default()
                            },
                            BackgroundColor(skin::button_danger()),
                            BorderColor::all(skin::border_soft()),
                            skin::shadow_card(),
                            UiTransform::IDENTITY,
                            crate::ui::anim::HoverMotion::default(),
                            DeleteButton,
                        ))
                        .with_children(|btn| {
                            btn.spawn((
                                Text::new(L10n::save_delete_selected(lang)),
                                TextFont {
                                    font: font.clone().into(),
                                    font_size: FontSize::from(skin::FONT_BODY + 2.0),
                                    ..default()
                                },
                                TextColor(skin::text_primary()),
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
    settings: Res<GameSettings>,
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

    let lang = settings.language;
    let font = skin::ui_font(&asset_server, lang);
    let cur = selected.0.clone();

    commands.entity(list_e).with_children(|parent| {
        if slots.slots.is_empty() {
            parent.spawn((
                Text::new(L10n::save_empty(lang)),
                TextFont {
                    font: font.clone().into(),
                    font_size: FontSize::from(18.0),
                    ..default()
                },
                TextColor(skin::text_muted()),
            ));
            return;
        }

        for meta in slots.slots.iter() {
            let is_selected = cur.as_deref() == Some(meta.file_name.as_str());
            let mut label = if meta.is_auto {
                format!("{}  ({})", meta.display_name, L10n::save_auto_tag(lang))
            } else {
                meta.display_name.clone()
            };
            if is_selected {
                label = format!("▶ {}   [{}]", label, L10n::save_selected_tag(lang));
            }

            parent
                .spawn((
                    Button,
                    Node {
                        width: Val::Percent(100.0),
                        height: Val::Px(44.0),
                        padding: UiRect::horizontal(Val::Px(12.0)),
                        justify_content: JustifyContent::SpaceBetween,
                        align_items: AlignItems::Center,
                        border: UiRect::all(Val::Px(1.0)),
                        border_radius: skin::radius(8.0),
                        ..default()
                    },
                    BackgroundColor(if is_selected {
                        skin::selected_fill()
                    } else {
                        skin::button_idle()
                    }),
                    BorderColor::all(if is_selected {
                        skin::accent_gold()
                    } else {
                        skin::border_soft()
                    }),
                    UiTransform::IDENTITY,
                    crate::ui::anim::HoverMotion::default(),
                    SaveSlotButton {
                        file_name: meta.file_name.clone(),
                        action: SaveSlotAction::Select,
                    },
                ))
                .with_children(|row| {
                    row.spawn((
                        Text::new(label),
                        TextFont {
                            font: font.clone().into(),
                            font_size: FontSize::from(18.0),
                            ..default()
                        },
                        TextColor(if is_selected {
                            skin::text_accent()
                        } else {
                            skin::text_primary()
                        }),
                    ));

                    if is_selected {
                        row.spawn((
                            Text::new("●"),
                            TextFont {
                                font: font.clone().into(),
                                font_size: FontSize::from(18.0),
                                ..default()
                            },
                            TextColor(skin::text_accent()),
                        ));
                    }
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
            SaveSlotAction::Save => skin::button_primary(),
            SaveSlotAction::Select => skin::button_idle(),
        };

        match *interaction {
            Interaction::Pressed => {
                bg.0 = skin::button_pressed();
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
            Interaction::Hovered => bg.0 = skin::button_hover(),
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
                bg.0 = skin::button_pressed();

                if let Some(name) = selected_slot.0.clone() {
                    load_tx.write(LoadSlotEvent { file_name: name });
                }

                if let Some(root) = q_overlay.iter().next() {
                    despawn_with_children(&mut commands, &children_q, root);
                }
            }
            Interaction::Hovered => bg.0 = skin::button_hover(),
            Interaction::None => bg.0 = skin::button_confirm(),
        }
    }
}

pub fn handle_delete_button(
    mut interactions: Query<
        (&Interaction, &mut BackgroundColor),
        (Changed<Interaction>, With<DeleteButton>),
    >,
    mut selected_slot: ResMut<SelectedSlot>,
    mut delete_tx: MessageWriter<DeleteSlotEvent>,
) {
    for (interaction, mut bg) in interactions.iter_mut() {
        match *interaction {
            Interaction::Pressed => {
                bg.0 = skin::button_pressed();
                if let Some(name) = selected_slot.0.clone() {
                    delete_tx.write(DeleteSlotEvent { file_name: name });
                    selected_slot.0 = None;
                }
            }
            Interaction::Hovered => bg.0 = skin::button_hover(),
            Interaction::None => bg.0 = skin::button_danger(),
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
    _asset_server: &AssetServer,
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
                height: Val::Px(46.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                border: UiRect::all(Val::Px(1.5)),
                border_radius: skin::radius(skin::BUTTON_RADIUS),
                ..default()
            },
            BackgroundColor(color),
            BorderColor::all(skin::border_soft()),
            skin::shadow_card(),
            UiTransform::IDENTITY,
            crate::ui::anim::HoverMotion::default(),
            button,
        ))
        .with_children(|btn| {
            btn.spawn((
                Text::new(label),
                TextFont {
                    font: font.clone().into(),
                    font_size: FontSize::from(20.0),
                    ..default()
                },
                TextColor(skin::text_primary()),
            ));
        });
}
