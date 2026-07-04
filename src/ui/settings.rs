use bevy::prelude::*;
use bevy::ui::{UiRect, Val};
use bevy::window::{MonitorSelection, PrimaryWindow, WindowMode};

use crate::i18n::{L10n, Language};
use crate::ui::EscBlockingUi;
use crate::ui::skin;
use crate::ui::types::{GameSettings, RESOLUTIONS};
use crate::utils::despawn_with_children;

#[derive(Resource)]
pub(super) struct SettingsOpenRequest;

#[derive(Component)]
pub(super) struct SettingsUiRoot;

#[derive(Component)]
pub(super) struct SettingsButton;

#[derive(Component)]
pub(super) struct ResolutionValue;

#[derive(Component)]
pub(super) struct ResolutionRow;

#[derive(Component)]
pub(super) struct VolumeValue;

#[derive(Component)]
pub(super) struct FullscreenValue;

#[derive(Component)]
pub(super) struct LanguageValue;

#[derive(Component, Clone, Copy)]
pub(super) enum SettingsAction {
    ResolutionPrev,
    ResolutionNext,
    VolumeDown,
    VolumeUp,
    LanguagePrev,
    LanguageNext,
    ToggleFullscreen,
    Apply,
    Close,
}

pub(super) fn open_settings_panel(commands: &mut Commands) {
    commands.insert_resource(SettingsOpenRequest);
}

pub(super) fn spawn_settings_panel_if_requested(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    request: Option<Res<SettingsOpenRequest>>,
    existing: Query<Entity, With<SettingsUiRoot>>,
    settings: Res<GameSettings>,
) {
    if request.is_none() {
        return;
    }

    commands.remove_resource::<SettingsOpenRequest>();

    if !existing.is_empty() {
        return;
    }

    let lang = settings.language;
    let font = skin::ui_font(&asset_server, lang);

    let (rw, rh) = current_resolution(&settings);
    let res_text = format!("{rw} x {rh}");
    let vol_text = format!("{:.0}%", (settings.volume * 100.0).clamp(0.0, 100.0));
    let fs_text = if settings.fullscreen {
        L10n::settings_on(lang)
    } else {
        L10n::settings_off(lang)
    }
    .to_string();
    let lang_text = L10n::language_name(lang).to_string();

    commands
        .spawn((
            SettingsUiRoot,
            EscBlockingUi,
            GlobalZIndex(300),
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(skin::overlay()),
        ))
        .with_children(|root| {
            root.spawn((
                Node {
                    width: Val::Px(820.0),
                    height: Val::Px(560.0),
                    flex_direction: FlexDirection::Column,
                    justify_content: JustifyContent::FlexStart,
                    align_items: AlignItems::Center,
                    padding: UiRect::all(Val::Px(30.0)),
                    border: UiRect::all(Val::Px(1.5)),
                    border_radius: skin::radius(skin::PANEL_RADIUS),
                    row_gap: Val::Px(18.0),
                    ..default()
                },
                skin::panel_decoration(),
                UiTransform::IDENTITY,
                crate::ui::anim::UiTween::pop_in(0.28, 0.95),
            ))
            .with_children(|panel| {
                panel.spawn((
                    Text::new(L10n::settings_title(lang)),
                    TextFont {
                        font: font.clone().into(),
                        font_size: FontSize::from(skin::FONT_TITLE),
                        ..default()
                    },
                    TextColor(skin::text_accent()),
                ));

                panel
                    .spawn((Node {
                        width: Val::Percent(100.0),
                        height: Val::Auto,
                        flex_direction: FlexDirection::Column,
                        justify_content: JustifyContent::FlexStart,
                        align_items: AlignItems::Stretch,
                        row_gap: Val::Px(14.0),
                        ..default()
                    },))
                    .with_children(|content| {
                        spawn_row_resolution(
                            content,
                            &asset_server,
                            &font,
                            res_text,
                            settings.fullscreen,
                            lang,
                        );
                        spawn_row_language(content, &asset_server, &font, lang_text, lang);
                        spawn_row_fullscreen(content, &asset_server, &font, fs_text, lang);
                        spawn_row_volume(content, &asset_server, &font, vol_text, lang);

                        content
                            .spawn((Node {
                                width: Val::Percent(100.0),
                                height: Val::Auto,
                                justify_content: JustifyContent::Center,
                                align_items: AlignItems::Center,
                                flex_direction: FlexDirection::Row,
                                column_gap: Val::Px(14.0),
                                padding: UiRect::top(Val::Px(18.0)),
                                ..default()
                            },))
                            .with_children(|buttons| {
                                spawn_action_button(
                                    buttons,
                                    &asset_server,
                                    &font,
                                    L10n::settings_apply(lang),
                                    SettingsAction::Apply,
                                );
                                spawn_action_button(
                                    buttons,
                                    &asset_server,
                                    &font,
                                    L10n::settings_back(lang),
                                    SettingsAction::Close,
                                );
                            });
                    });
            });
        });
}

pub(super) fn handle_settings_buttons(
    mut interactions: Query<
        (&Interaction, &mut BackgroundColor, &SettingsAction),
        (Changed<Interaction>, With<Button>, With<SettingsButton>),
    >,
    mut settings: ResMut<GameSettings>,
    mut window_q: Query<&mut Window, With<PrimaryWindow>>,
    root_q: Query<Entity, With<SettingsUiRoot>>,
    children_q: Query<&Children>,
    mut commands: Commands,
) {
    let mut reopen_ui = false;
    for (interaction, mut bg, action) in &mut interactions {
        match *interaction {
            Interaction::Pressed => {
                bg.0 = skin::button_pressed();

                match *action {
                    SettingsAction::ResolutionPrev => {
                        if settings.fullscreen {
                            continue;
                        }
                        step_resolution(&mut settings, -1);
                        apply_window_settings(&settings, &mut window_q);
                    }
                    SettingsAction::ResolutionNext => {
                        if settings.fullscreen {
                            continue;
                        }
                        step_resolution(&mut settings, 1);
                        apply_window_settings(&settings, &mut window_q);
                    }
                    SettingsAction::VolumeDown => {
                        settings.volume = (settings.volume - 0.05).clamp(0.0, 1.0);
                    }
                    SettingsAction::VolumeUp => {
                        settings.volume = (settings.volume + 0.05).clamp(0.0, 1.0);
                    }
                    SettingsAction::LanguagePrev => {
                        settings.language = settings.language.cycle(-1);
                        reopen_ui = true;
                    }
                    SettingsAction::LanguageNext => {
                        settings.language = settings.language.cycle(1);
                        reopen_ui = true;
                    }
                    SettingsAction::ToggleFullscreen => {
                        settings.fullscreen = !settings.fullscreen;
                        apply_window_settings(&settings, &mut window_q);
                    }
                    SettingsAction::Apply => {
                        apply_window_settings(&settings, &mut window_q);
                    }
                    SettingsAction::Close => {
                        close_settings_ui(&mut commands, &root_q, &children_q);
                    }
                }
            }
            Interaction::Hovered => bg.0 = skin::button_hover(),
            Interaction::None => bg.0 = skin::button_idle(),
        }
    }

    if reopen_ui {
        close_settings_ui(&mut commands, &root_q, &children_q);
        open_settings_panel(&mut commands);
    }
}

pub(super) fn sync_settings_texts(
    settings: Res<GameSettings>,
    mut q: Query<(
        &mut Text,
        AnyOf<(
            &ResolutionValue,
            &VolumeValue,
            &FullscreenValue,
            &LanguageValue,
        )>,
    )>,
) {
    if !settings.is_changed() {
        return;
    }

    let (rw, rh) = current_resolution(&settings);
    let res_text = format!("{rw} x {rh}");
    let vol_text = format!("{:.0}%", (settings.volume * 100.0).clamp(0.0, 100.0));
    let fs_text = if settings.fullscreen {
        L10n::settings_on(settings.language)
    } else {
        L10n::settings_off(settings.language)
    }
    .to_string();
    let lang_text = L10n::language_name(settings.language).to_string();

    for (mut text, (is_res, is_vol, is_fs, is_lang)) in &mut q {
        if is_res.is_some() {
            text.0 = res_text.clone();
        } else if is_vol.is_some() {
            text.0 = vol_text.clone();
        } else if is_fs.is_some() {
            text.0 = fs_text.clone();
        } else if is_lang.is_some() {
            text.0 = lang_text.clone();
        }
    }
}

pub(super) fn sync_settings_resolution_row_visibility(
    settings: Res<GameSettings>,
    mut rows: Query<&mut Node, With<ResolutionRow>>,
) {
    if !settings.is_changed() {
        return;
    }

    for mut node in &mut rows {
        node.display = if settings.fullscreen {
            Display::None
        } else {
            Display::Flex
        };
    }
}

pub(super) fn close_settings_on_esc(
    keyboard: Res<ButtonInput<KeyCode>>,
    root_q: Query<Entity, With<SettingsUiRoot>>,
    children_q: Query<&Children>,
    mut commands: Commands,
) {
    if !keyboard.just_pressed(KeyCode::Escape) {
        return;
    }
    close_settings_ui(&mut commands, &root_q, &children_q);
}

fn close_settings_ui(
    commands: &mut Commands,
    root_q: &Query<Entity, With<SettingsUiRoot>>,
    children_q: &Query<&Children>,
) {
    if let Ok(root) = root_q.single() {
        despawn_with_children(commands, children_q, root);
    }
}

fn current_resolution(settings: &GameSettings) -> (u32, u32) {
    if RESOLUTIONS.is_empty() {
        return (1280, 720);
    }
    let idx = settings.resolution_index % RESOLUTIONS.len();
    RESOLUTIONS[idx]
}

fn step_resolution(settings: &mut GameSettings, dir: i32) {
    let len = RESOLUTIONS.len();
    if len == 0 {
        settings.resolution_index = 0;
        return;
    }

    let cur = settings.resolution_index % len;
    let next = if dir >= 0 {
        (cur + 1) % len
    } else {
        (cur + len - 1) % len
    };
    settings.resolution_index = next;
}

fn apply_window_settings(
    settings: &GameSettings,
    window_q: &mut Query<&mut Window, With<PrimaryWindow>>,
) {
    let Ok(mut window) = window_q.single_mut() else {
        return;
    };

    if settings.fullscreen {
        window.mode = WindowMode::BorderlessFullscreen(MonitorSelection::Current);
    } else {
        window.mode = WindowMode::Windowed;
        let (w, h) = current_resolution(settings);
        window.resolution.set(w as f32, h as f32);
    }
}

fn spawn_row_resolution(
    parent: &mut ChildSpawnerCommands<'_>,
    asset_server: &AssetServer,
    font: &Handle<Font>,
    value: String,
    fullscreen: bool,
    lang: Language,
) {
    parent
        .spawn((
            ResolutionRow,
            Node {
                width: Val::Percent(100.0),
                height: Val::Auto,
                display: if fullscreen {
                    Display::None
                } else {
                    Display::Flex
                },
                flex_direction: FlexDirection::Row,
                justify_content: JustifyContent::SpaceBetween,
                align_items: AlignItems::Center,
                ..default()
            },
        ))
        .with_children(|row| {
            row.spawn((
                Text::new(L10n::settings_resolution(lang)),
                TextFont {
                    font: font.clone().into(),
                    font_size: FontSize::from(24.0),
                    ..default()
                },
                TextColor(skin::text_primary()),
            ));

            row.spawn((
                Text::new(value),
                TextFont {
                    font: font.clone().into(),
                    font_size: FontSize::from(24.0),
                    ..default()
                },
                TextColor(skin::text_muted()),
                ResolutionValue,
            ));

            row.spawn(Node {
                flex_direction: FlexDirection::Row,
                justify_content: JustifyContent::FlexEnd,
                align_items: AlignItems::Center,
                column_gap: Val::Px(10.0),
                ..default()
            })
            .with_children(|btns| {
                spawn_action_button(
                    btns,
                    asset_server,
                    font,
                    "←",
                    SettingsAction::ResolutionPrev,
                );
                spawn_action_button(
                    btns,
                    asset_server,
                    font,
                    "→",
                    SettingsAction::ResolutionNext,
                );
            });
        });
}

fn spawn_row_language(
    parent: &mut ChildSpawnerCommands<'_>,
    asset_server: &AssetServer,
    font: &Handle<Font>,
    value: String,
    lang: Language,
) {
    spawn_row(
        parent,
        asset_server,
        font,
        L10n::settings_language(lang),
        value,
        LanguageValue,
        Some((SettingsAction::LanguagePrev, "←")),
        Some((SettingsAction::LanguageNext, "→")),
        None,
    );
}

fn spawn_row_fullscreen(
    parent: &mut ChildSpawnerCommands<'_>,
    asset_server: &AssetServer,
    font: &Handle<Font>,
    value: String,
    lang: Language,
) {
    spawn_row(
        parent,
        asset_server,
        font,
        L10n::settings_fullscreen(lang),
        value,
        FullscreenValue,
        Some((
            SettingsAction::ToggleFullscreen,
            L10n::settings_toggle(lang),
        )),
        None,
        None,
    );
}

fn spawn_row_volume(
    parent: &mut ChildSpawnerCommands<'_>,
    asset_server: &AssetServer,
    font: &Handle<Font>,
    value: String,
    lang: Language,
) {
    spawn_row(
        parent,
        asset_server,
        font,
        L10n::settings_volume(lang),
        value,
        VolumeValue,
        Some((SettingsAction::VolumeDown, "-")),
        Some((SettingsAction::VolumeUp, "+")),
        None,
    );
}

fn spawn_row<M: Component>(
    parent: &mut ChildSpawnerCommands<'_>,
    asset_server: &AssetServer,
    font: &Handle<Font>,
    label: &str,
    value: String,
    marker: M,
    left: Option<(SettingsAction, &str)>,
    right: Option<(SettingsAction, &str)>,
    extra: Option<(SettingsAction, &str)>,
) {
    parent
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Auto,
                flex_direction: FlexDirection::Row,
                justify_content: JustifyContent::SpaceBetween,
                align_items: AlignItems::Center,
                padding: UiRect::axes(Val::Px(16.0), Val::Px(10.0)),
                border: UiRect::all(Val::Px(1.0)),
                border_radius: skin::radius(10.0),
                ..default()
            },
            skin::subpanel_decoration(),
        ))
        .with_children(|row| {
            row.spawn((
                Text::new(label),
                TextFont {
                    font: font.clone().into(),
                    font_size: FontSize::from(24.0),
                    ..default()
                },
                TextColor(skin::text_primary()),
            ));

            row.spawn((
                Text::new(value),
                TextFont {
                    font: font.clone().into(),
                    font_size: FontSize::from(24.0),
                    ..default()
                },
                TextColor(skin::text_muted()),
                marker,
            ));

            row.spawn(Node {
                flex_direction: FlexDirection::Row,
                justify_content: JustifyContent::FlexEnd,
                align_items: AlignItems::Center,
                column_gap: Val::Px(10.0),
                ..default()
            })
            .with_children(|btns| {
                if let Some((a, t)) = left {
                    spawn_action_button(btns, asset_server, font, t, a);
                }
                if let Some((a, t)) = right {
                    spawn_action_button(btns, asset_server, font, t, a);
                }
                if let Some((a, t)) = extra {
                    spawn_action_button(btns, asset_server, font, t, a);
                }
            });
        });
}

fn spawn_action_button(
    parent: &mut ChildSpawnerCommands<'_>,
    _asset_server: &AssetServer,
    font: &Handle<Font>,
    text: &str,
    action: SettingsAction,
) {
    parent
        .spawn((
            Button,
            SettingsButton,
            action,
            Node {
                width: Val::Px(116.0),
                height: Val::Px(44.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                border: UiRect::all(Val::Px(1.5)),
                border_radius: skin::radius(skin::BUTTON_RADIUS),
                ..default()
            },
            BackgroundColor(skin::button_idle()),
            BorderColor::all(skin::border_soft()),
            skin::shadow_card(),
            UiTransform::IDENTITY,
            crate::ui::anim::HoverMotion::default(),
        ))
        .with_children(|b| {
            b.spawn((
                Text::new(text),
                TextFont {
                    font: font.clone().into(),
                    font_size: FontSize::from(skin::FONT_BODY + 2.0),
                    ..default()
                },
                TextColor(skin::text_primary()),
            ));
        });
}
