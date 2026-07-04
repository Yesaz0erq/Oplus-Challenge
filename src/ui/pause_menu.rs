use bevy::prelude::*;
use bevy::ui::Val;

use crate::i18n::L10n;
use crate::state::GameState;
use crate::ui::EscBlockingUi;
use crate::ui::skin;
use crate::ui::types::GameSettings;

#[derive(Component)]
pub struct PauseMenuUI;

#[derive(Resource, Default)]
pub struct SuppressPauseMenuOnce(pub bool);

#[derive(Component, Clone, Copy)]
pub(crate) enum PauseMenuAction {
    Resume,
    Save,
    Settings,
    BackToMainMenu,
}

pub fn spawn_pause_menu(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    settings: Res<GameSettings>,
    mut suppress_once: ResMut<SuppressPauseMenuOnce>,
) {
    if suppress_once.0 {
        suppress_once.0 = false;
        return;
    }

    let lang = settings.language;
    let font = skin::ui_font(&asset_server, lang);

    commands
        .spawn((
            PauseMenuUI,
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(16.0),
                ..default()
            },
            BackgroundColor(skin::overlay()),
        ))
        .with_children(|parent| {
            parent
                .spawn((
                    Node {
                        width: Val::Px(360.0),
                        padding: UiRect::axes(Val::Px(30.0), Val::Px(24.0)),
                        border: UiRect::all(Val::Px(1.5)),
                        border_radius: skin::radius(skin::PANEL_RADIUS),
                        flex_direction: FlexDirection::Column,
                        align_items: AlignItems::Stretch,
                        row_gap: Val::Px(14.0),
                        ..default()
                    },
                    skin::panel_decoration(),
                    UiTransform::IDENTITY,
                    crate::ui::anim::UiTween::pop_in(0.26, 0.95),
                ))
                .with_children(|panel| {
                    parent_title(panel, &font, L10n::pause_resume(lang));
                    skin::spawn_text_button(
                        panel,
                        &font,
                        L10n::pause_resume(lang),
                        skin::ButtonKind::Primary,
                        PauseMenuAction::Resume,
                    );
                    skin::spawn_text_button(
                        panel,
                        &font,
                        L10n::main_menu_save(lang),
                        skin::ButtonKind::Neutral,
                        PauseMenuAction::Save,
                    );
                    skin::spawn_text_button(
                        panel,
                        &font,
                        L10n::main_menu_settings(lang),
                        skin::ButtonKind::Confirm,
                        PauseMenuAction::Settings,
                    );
                    skin::spawn_text_button(
                        panel,
                        &font,
                        L10n::pause_back_to_menu(lang),
                        skin::ButtonKind::Danger,
                        PauseMenuAction::BackToMainMenu,
                    );
                });
        });
}

fn parent_title(parent: &mut ChildSpawnerCommands<'_>, font: &Handle<Font>, _label: &str) {
    parent.spawn((
        Text::new("PAUSED"),
        TextFont {
            font: font.clone().into(),
            font_size: FontSize::from(skin::FONT_HEADING),
            ..default()
        },
        TextColor(skin::text_accent()),
        Node {
            margin: UiRect::bottom(Val::Px(6.0)),
            align_self: AlignSelf::Center,
            ..default()
        },
    ));
}

pub fn cleanup_pause_menu(mut commands: Commands, q: Query<Entity, With<PauseMenuUI>>) {
    if let Ok(e) = q.single() {
        commands.entity(e).try_despawn();
    }
}

pub fn handle_pause_menu_buttons(
    mut interactions: Query<
        (&Interaction, &mut BackgroundColor, &PauseMenuAction),
        Changed<Interaction>,
    >,
    blocking_ui_q: Query<Entity, With<EscBlockingUi>>,
    mut next_state: ResMut<NextState<GameState>>,
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    settings: Res<GameSettings>,
) {
    if !blocking_ui_q.is_empty() {
        return;
    }

    for (interaction, mut bg, action) in &mut interactions {
        match *interaction {
            Interaction::Pressed => {
                bg.0 = skin::button_pressed();
                match action {
                    PauseMenuAction::Resume => next_state.set(GameState::InGame),
                    PauseMenuAction::Save => crate::ui::save::open_save_panel(
                        &mut commands,
                        &asset_server,
                        settings.language,
                    ),
                    PauseMenuAction::Settings => {
                        crate::ui::settings::open_settings_panel(&mut commands)
                    }
                    PauseMenuAction::BackToMainMenu => next_state.set(GameState::MainMenu),
                }
            }
            Interaction::Hovered => bg.0 = skin::button_hover(),
            Interaction::None => bg.0 = skin::button_idle(),
        }
    }
}
