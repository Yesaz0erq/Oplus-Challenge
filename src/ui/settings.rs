use bevy::prelude::*;
use bevy::ui::{UiRect, Val};
use bevy::window::{MonitorSelection, PrimaryWindow, WindowMode};

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
pub(super) struct VolumeValue;

#[derive(Component)]
pub(super) struct FullscreenValue;

#[derive(Component, Clone, Copy)]
pub(super) enum SettingsAction {
    ResolutionPrev,
    ResolutionNext,
    VolumeDown,
    VolumeUp,
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

    let bg: Handle<Image> = asset_server.load("settings.png");
    let font: Handle<Font> = asset_server.load("fonts/YuFanLixing.otf");

    let (rw, rh) = current_resolution(&settings);
    let res_text = format!("{rw} x {rh}");
    let vol_text = format!("{:.0}%", (settings.volume * 100.0).clamp(0.0, 100.0));
    let fs_text = if settings.fullscreen { "开" } else { "关" }.to_string();

    commands
        .spawn((
            SettingsUiRoot,
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.6)),
        ))
        .with_children(|root| {
            root.spawn((
                Node {
                    width: Val::Px(820.0),
                    height: Val::Px(560.0),
                    flex_direction: FlexDirection::Column,
                    justify_content: JustifyContent::FlexStart,
                    align_items: AlignItems::Center,
                    padding: UiRect::all(Val::Px(28.0)),
                    row_gap: Val::Px(18.0),
                    ..default()
                },
                ImageNode::new(bg),
            ))
            .with_children(|panel| {
                panel.spawn((
                    Text::new("设置"),
                    TextFont {
                        font: font.clone(),
                        font_size: 40.0,
                        ..default()
                    },
                    TextColor(Color::WHITE),
                ));

                panel.spawn((
                    Node {
                        width: Val::Percent(100.0),
                        height: Val::Auto,
                        flex_direction: FlexDirection::Column,
                        justify_content: JustifyContent::FlexStart,
                        align_items: AlignItems::Stretch,
                        row_gap: Val::Px(14.0),
                        ..default()
                    },
                ))
                .with_children(|content| {
                    spawn_row_resolution(content, &font, res_text);
                    spawn_row_fullscreen(content, &font, fs_text);
                    spawn_row_volume(content, &font, vol_text);

                    content
                        .spawn((
                            Node {
                                width: Val::Percent(100.0),
                                height: Val::Auto,
                                justify_content: JustifyContent::Center,
                                align_items: AlignItems::Center,
                                flex_direction: FlexDirection::Row,
                                column_gap: Val::Px(14.0),
                                padding: UiRect::top(Val::Px(18.0)),
                                ..default()
                            },
                        ))
                        .with_children(|buttons| {
                            spawn_action_button(buttons, &font, "应用", SettingsAction::Apply);
                            spawn_action_button(buttons, &font, "返回", SettingsAction::Close);
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
    for (interaction, mut bg, action) in &mut interactions {
        match *interaction {
            Interaction::Pressed => {
                bg.0 = Color::srgb(0.85, 0.85, 0.95);

                match *action {
                    SettingsAction::ResolutionPrev => {
                        step_resolution(&mut settings, -1);
                        apply_window_settings(&settings, &mut window_q);
                    }
                    SettingsAction::ResolutionNext => {
                        step_resolution(&mut settings, 1);
                        apply_window_settings(&settings, &mut window_q);
                    }
                    SettingsAction::VolumeDown => {
                        settings.volume = (settings.volume - 0.05).clamp(0.0, 1.0);
                    }
                    SettingsAction::VolumeUp => {
                        settings.volume = (settings.volume + 0.05).clamp(0.0, 1.0);
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
            Interaction::Hovered => bg.0 = Color::srgb(0.55, 0.55, 0.7),
            Interaction::None => bg.0 = Color::srgb(0.25, 0.25, 0.35),
        }
    }
}

pub(super) fn sync_settings_texts(
    settings: Res<GameSettings>,
    mut q: Query<(&mut Text, AnyOf<(&ResolutionValue, &VolumeValue, &FullscreenValue)>)>,
) {
    if !settings.is_changed() {
        return;
    }

    let (rw, rh) = current_resolution(&settings);
    let res_text = format!("{rw} x {rh}");
    let vol_text = format!("{:.0}%", (settings.volume * 100.0).clamp(0.0, 100.0));
    let fs_text = if settings.fullscreen { "开" } else { "关" }.to_string();

    for (mut text, (is_res, is_vol, is_fs)) in &mut q {
        if is_res.is_some() {
            text.0 = res_text.clone();
        } else if is_vol.is_some() {
            text.0 = vol_text.clone();
        } else if is_fs.is_some() {
            text.0 = fs_text.clone();
        }
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

fn close_settings_ui(commands: &mut Commands, root_q: &Query<Entity, With<SettingsUiRoot>>, children_q: &Query<&Children>) {
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

fn apply_window_settings(settings: &GameSettings, window_q: &mut Query<&mut Window, With<PrimaryWindow>>) {
    let Ok(mut window) = window_q.single_mut() else { return; };

    if settings.fullscreen {
        window.mode = WindowMode::BorderlessFullscreen(MonitorSelection::Current);
    } else {
        window.mode = WindowMode::Windowed;
        let (w, h) = current_resolution(settings);
        window.resolution.set(w as f32, h as f32);
    }
}

fn spawn_row_resolution(parent: &mut ChildSpawnerCommands<'_>, font: &Handle<Font>, value: String) {
    spawn_row(
        parent,
        font,
        "分辨率",
        value,
        ResolutionValue,
        Some((SettingsAction::ResolutionPrev, "←")),
        Some((SettingsAction::ResolutionNext, "→")),
        None,
    );
}

fn spawn_row_fullscreen(parent: &mut ChildSpawnerCommands<'_>, font: &Handle<Font>, value: String) {
    spawn_row(
        parent,
        font,
        "全屏",
        value,
        FullscreenValue,
        Some((SettingsAction::ToggleFullscreen, "切换")),
        None,
        None,
    );
}

fn spawn_row_volume(parent: &mut ChildSpawnerCommands<'_>, font: &Handle<Font>, value: String) {
    spawn_row(
        parent,
        font,
        "音量",
        value,
        VolumeValue,
        Some((SettingsAction::VolumeDown, "-")),
        Some((SettingsAction::VolumeUp, "+")),
        None,
    );
}

fn spawn_row<M: Component>(
    parent: &mut ChildSpawnerCommands<'_>,
    font: &Handle<Font>,
    label: &str,
    value: String,
    marker: M,
    left: Option<(SettingsAction, &str)>,
    right: Option<(SettingsAction, &str)>,
    extra: Option<(SettingsAction, &str)>,
) {
    parent
        .spawn(Node {
            width: Val::Percent(100.0),
            height: Val::Auto,
            flex_direction: FlexDirection::Row,
            justify_content: JustifyContent::SpaceBetween,
            align_items: AlignItems::Center,
            ..default()
        })
        .with_children(|row| {
            row.spawn((
                Text::new(label),
                TextFont {
                    font: font.clone(),
                    font_size: 24.0,
                    ..default()
                },
                TextColor(Color::WHITE),
            ));

            row.spawn((
                Text::new(value),
                TextFont {
                    font: font.clone(),
                    font_size: 24.0,
                    ..default()
                },
                TextColor(Color::WHITE),
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
                    spawn_action_button(btns, font, t, a);
                }
                if let Some((a, t)) = right {
                    spawn_action_button(btns, font, t, a);
                }
                if let Some((a, t)) = extra {
                    spawn_action_button(btns, font, t, a);
                }
            });
        });
}

fn spawn_action_button(
    parent: &mut ChildSpawnerCommands<'_>,
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
                width: Val::Px(110.0),
                height: Val::Px(42.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(Color::srgb(0.25, 0.25, 0.35)),
        ))
        .with_children(|b| {
            b.spawn((
                Text::new(text),
                TextFont {
                    font: font.clone(),
                    font_size: 20.0,
                    ..default()
                },
                TextColor(Color::WHITE),
            ));
        });
}
