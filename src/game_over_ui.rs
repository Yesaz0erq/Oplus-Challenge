use bevy::prelude::*;
use bevy::ui::Val;

use crate::enemy::Enemy;
use crate::i18n::L10n;
use crate::save::{
    CurrentSlot, LoadSlotEvent, PendingLoad, SaveSlots, refresh_save_slots_from_disk,
};
use crate::state::GameState;
use crate::ui::skin;
use crate::ui::types::GameSettings;
use crate::utils::despawn_with_children;

pub struct GameOverUiPlugin;

#[derive(Component)]
pub struct GameOverRoot;

#[derive(Component)]
pub enum GameOverButton {
    BackToMainMenu,
}

#[derive(Component)]
pub struct ManualSaveSlotButton {
    pub file_name: String,
}

impl Plugin for GameOverUiPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            OnEnter(GameState::GameOver),
            (
                reset_after_game_over,
                setup_game_over_ui.after(reset_after_game_over),
            ),
        )
        .add_systems(OnExit(GameState::GameOver), cleanup_game_over_ui)
        .add_systems(
            Update,
            (handle_game_over_buttons, handle_manual_save_slot_buttons)
                .run_if(in_state(GameState::GameOver)),
        );
    }
}

fn reset_after_game_over(
    mut commands: Commands,
    mut slots: ResMut<SaveSlots>,
    mut pending: ResMut<PendingLoad>,
    mut current: ResMut<CurrentSlot>,
    enemies: Query<Entity, With<Enemy>>,
) {
    for e in enemies.iter() {
        commands.entity(e).despawn();
    }

    pending.file_name = None;
    current.file_name = None;

    refresh_save_slots_from_disk(&mut slots);
}

fn setup_game_over_ui(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    slots: Res<SaveSlots>,
    settings: Res<GameSettings>,
) {
    let lang = settings.language;
    let font = skin::ui_font(&asset_server, lang);

    let mut manual_slots: Vec<_> = slots.slots.iter().filter(|s| !s.is_auto).collect();
    manual_slots.reverse();
    manual_slots.truncate(8);

    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                top: Val::Px(0.0),
                left: Val::Px(0.0),
                right: Val::Px(0.0),
                bottom: Val::Px(0.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(skin::overlay()),
            GameOverRoot,
        ))
        .with_children(|parent| {
            parent
                .spawn((
                    Node {
                        width: Val::Px(720.0),
                        padding: UiRect::all(Val::Px(30.0)),
                        border: UiRect::all(Val::Px(1.5)),
                        border_radius: skin::radius(skin::PANEL_RADIUS),
                        flex_direction: FlexDirection::Column,
                        row_gap: Val::Px(14.0),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        ..default()
                    },
                    skin::panel_decoration(),
                    UiTransform::IDENTITY,
                    crate::ui::anim::UiTween::pop_in(0.34, 0.92),
                ))
                .with_children(|panel| {
                    panel.spawn((
                        Text::new(L10n::game_over_title(lang)),
                        TextFont {
                            font: font.clone().into(),
                            font_size: FontSize::from(skin::FONT_TITLE),
                            ..default()
                        },
                        TextColor(skin::button_danger().lighter(0.25)),
                        TextShadow {
                            offset: Vec2::new(0.0, 3.0),
                            color: Color::srgba(0.0, 0.0, 0.0, 0.7),
                        },
                    ));

                    panel.spawn((
                        Text::new(L10n::game_over_desc(lang)),
                        TextFont {
                            font: font.clone().into(),
                            font_size: FontSize::from(18.0),
                            ..default()
                        },
                        TextColor(skin::text_muted()),
                    ));

                    panel
                        .spawn(Node {
                            width: Val::Px(640.0),
                            flex_direction: FlexDirection::Column,
                            row_gap: Val::Px(10.0),
                            margin: UiRect::top(Val::Px(10.0)),
                            ..default()
                        })
                        .with_children(|list| {
                            if manual_slots.is_empty() {
                                list.spawn((
                                    Text::new(L10n::game_over_no_manual_saves(lang)),
                                    TextFont {
                                        font: font.clone().into(),
                                        font_size: FontSize::from(18.0),
                                        ..default()
                                    },
                                    TextColor(skin::text_muted()),
                                ));
                            } else {
                                for s in manual_slots {
                                    list.spawn((
                                        Button,
                                        Node {
                                            width: Val::Px(640.0),
                                            height: Val::Px(48.0),
                                            padding: UiRect::horizontal(Val::Px(16.0)),
                                            border: UiRect::all(Val::Px(1.5)),
                                            border_radius: skin::radius(skin::BUTTON_RADIUS),
                                            justify_content: JustifyContent::SpaceBetween,
                                            align_items: AlignItems::Center,
                                            ..default()
                                        },
                                        BackgroundColor(skin::button_idle()),
                                        BorderColor::all(skin::border_soft()),
                                        skin::shadow_card(),
                                        UiTransform::IDENTITY,
                                        crate::ui::anim::HoverMotion::default(),
                                        ManualSaveSlotButton {
                                            file_name: s.file_name.clone(),
                                        },
                                    ))
                                    .with_children(|btn| {
                                        btn.spawn((
                                            Text::new(format!(
                                                "{}: {}",
                                                L10n::save_entry_prefix(lang),
                                                s.display_name
                                            )),
                                            TextFont {
                                                font: font.clone().into(),
                                                font_size: FontSize::from(18.0),
                                                ..default()
                                            },
                                            TextColor(skin::text_primary()),
                                        ));
                                        btn.spawn((
                                            Text::new(L10n::game_over_load_restart(lang)),
                                            TextFont {
                                                font: font.clone().into(),
                                                font_size: FontSize::from(16.0),
                                                ..default()
                                            },
                                            TextColor(skin::text_muted()),
                                        ));
                                    });
                                }
                            }
                        });

                    panel
                        .spawn(Node {
                            flex_direction: FlexDirection::Row,
                            column_gap: Val::Px(14.0),
                            margin: UiRect::top(Val::Px(16.0)),
                            ..default()
                        })
                        .with_children(|row| {
                            row.spawn((
                                Button,
                                Node {
                                    width: Val::Px(220.0),
                                    height: Val::Px(50.0),
                                    justify_content: JustifyContent::Center,
                                    align_items: AlignItems::Center,
                                    border: UiRect::all(Val::Px(1.5)),
                                    border_radius: skin::radius(skin::BUTTON_RADIUS),
                                    ..default()
                                },
                                BackgroundColor(skin::button_primary()),
                                BorderColor::all(skin::border_soft()),
                                skin::shadow_card(),
                                UiTransform::IDENTITY,
                                crate::ui::anim::HoverMotion::default(),
                                GameOverButton::BackToMainMenu,
                            ))
                            .with_children(|btn| {
                                btn.spawn((
                                    Text::new(L10n::game_over_back_to_title(lang)),
                                    TextFont {
                                        font: font.clone().into(),
                                        font_size: FontSize::from(20.0),
                                        ..default()
                                    },
                                    TextColor(skin::text_primary()),
                                ));
                            });
                        });
                });
        });
}

fn handle_manual_save_slot_buttons(
    mut commands: Commands,
    mut q: Query<
        (&Interaction, &mut BackgroundColor, &ManualSaveSlotButton),
        (Changed<Interaction>, With<Button>),
    >,
    mut load_tx: MessageWriter<LoadSlotEvent>,
    mut next_state: ResMut<NextState<GameState>>,
    enemies: Query<Entity, With<Enemy>>,
) {
    for (interaction, mut bg, btn) in q.iter_mut() {
        match *interaction {
            Interaction::Pressed => {
                bg.0 = skin::button_pressed();

                for e in enemies.iter() {
                    commands.entity(e).despawn();
                }

                load_tx.write(LoadSlotEvent {
                    file_name: btn.file_name.clone(),
                });
                next_state.set(GameState::InGame);
            }
            Interaction::Hovered => bg.0 = skin::button_hover(),
            Interaction::None => bg.0 = skin::button_idle(),
        }
    }
}

fn handle_game_over_buttons(
    mut next_state: ResMut<NextState<GameState>>,
    mut q: Query<
        (&Interaction, &mut BackgroundColor, &GameOverButton),
        (Changed<Interaction>, With<Button>),
    >,
) {
    for (interaction, mut bg, button) in q.iter_mut() {
        match *interaction {
            Interaction::Pressed => {
                bg.0 = skin::button_pressed();
                match button {
                    GameOverButton::BackToMainMenu => next_state.set(GameState::MainMenu),
                }
            }
            Interaction::Hovered => bg.0 = skin::button_hover(),
            Interaction::None => bg.0 = skin::button_primary(),
        }
    }
}

fn cleanup_game_over_ui(
    mut commands: Commands,
    roots: Query<Entity, With<GameOverRoot>>,
    children_q: Query<&Children>,
) {
    for root in roots.iter() {
        despawn_with_children(&mut commands, &children_q, root);
    }
}
