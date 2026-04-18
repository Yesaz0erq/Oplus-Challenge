use bevy::image::ImageSampler;
use bevy::prelude::*;
use bevy::ui::Val;
use bevy::window::PrimaryWindow;

use crate::i18n::L10n;
use crate::state::GameState;
use crate::ui::skin;
use crate::ui::types::GameSettings;

#[derive(Component)]
pub struct MainMenuUI;

#[derive(Component)]
pub struct MainMenuBackground;

#[derive(Component)]
pub struct MainMenuBackgroundFade {
    timer: Timer,
}

#[derive(Component)]
pub struct MainMenuBackgroundConfigured;

#[derive(Component, Clone, Copy)]
pub enum MainMenuAction {
    Start,
    Save,
    Settings,
    Exit,
}

pub fn spawn_main_menu(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    settings: Res<GameSettings>,
) {
    let font = asset_server.load("fonts/YuFanLixing.otf");
    let lang = settings.language;

    let bg_handle: Handle<Image> = asset_server.load("main_background.png");
    commands.spawn((
        MainMenuBackground,
        MainMenuBackgroundFade {
            timer: Timer::from_seconds(0.45, TimerMode::Once),
        },
        GlobalZIndex(-100),
        Node {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            position_type: PositionType::Absolute,
            left: Val::Px(0.0),
            top: Val::Px(0.0),
            ..default()
        },
        BackgroundColor(Color::BLACK),
        ImageNode::new(bg_handle).with_color(Color::srgba(1.0, 1.0, 1.0, 0.0)),
    ));

    commands
        .spawn((
            MainMenuUI,
            GlobalZIndex(10),
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(16.0),
                ..default()
            },
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.0)),
        ))
        .with_children(|parent| {
            parent
                .spawn((
                    Node {
                        width: Val::Px(360.0),
                        padding: UiRect::axes(Val::Px(30.0), Val::Px(26.0)),
                        flex_direction: FlexDirection::Column,
                        align_items: AlignItems::Stretch,
                        row_gap: Val::Px(14.0),
                        ..default()
                    },
                    BackgroundColor(skin::panel_tint()),
                    ImageNode::new(skin::panel(&asset_server)),
                ))
                .with_children(|panel| {
                    spawn_button(
                        panel,
                        &asset_server,
                        &font,
                        L10n::main_menu_start(lang),
                        MainMenuAction::Start,
                    );
                    spawn_button(
                        panel,
                        &asset_server,
                        &font,
                        L10n::main_menu_save(lang),
                        MainMenuAction::Save,
                    );
                    spawn_button(
                        panel,
                        &asset_server,
                        &font,
                        L10n::main_menu_settings(lang),
                        MainMenuAction::Settings,
                    );
                    spawn_button(
                        panel,
                        &asset_server,
                        &font,
                        L10n::main_menu_exit(lang),
                        MainMenuAction::Exit,
                    );
                });
        });
}

pub fn cleanup_main_menu(
    mut commands: Commands,
    q_ui: Query<Entity, With<MainMenuUI>>,
    q_bg: Query<Entity, With<MainMenuBackground>>,
) {
    for e in q_ui.iter() {
        commands.entity(e).try_despawn();
    }
    for e in q_bg.iter() {
        commands.entity(e).try_despawn();
    }
}

pub fn animate_main_menu_fade(
    time: Res<Time>,
    mut commands: Commands,
    mut q: Query<(Entity, &mut MainMenuBackgroundFade, &mut ImageNode)>,
) {
    for (entity, mut fade, mut image) in &mut q {
        fade.timer.tick(time.delta());
        let t = fade.timer.fraction();
        let eased = t * t * (3.0 - 2.0 * t);
        image.color = Color::srgba(1.0, 1.0, 1.0, eased);
        if fade.timer.is_finished() {
            commands.entity(entity).remove::<MainMenuBackgroundFade>();
        }
    }
}

pub fn sync_main_menu_background_cover(
    mut commands: Commands,
    windows: Query<&Window, With<PrimaryWindow>>,
    mut images: ResMut<Assets<Image>>,
    mut q: Query<
        (
            Entity,
            &mut Node,
            &ImageNode,
            Option<&MainMenuBackgroundConfigured>,
        ),
        With<MainMenuBackground>,
    >,
) {
    let Ok(window) = windows.single() else {
        return;
    };
    let win_w = window.resolution.width();
    let win_h = window.resolution.height();
    if win_w <= 0.0 || win_h <= 0.0 {
        return;
    }

    for (entity, mut node, image_node, configured) in &mut q {
        let Some(image) = images.get_mut(&image_node.image) else {
            continue;
        };

        if configured.is_none() {
            image.sampler = ImageSampler::linear();
            commands.entity(entity).insert(MainMenuBackgroundConfigured);
        }

        let img_size = image.size();
        let img_w = img_size.x as f32;
        let img_h = img_size.y as f32;
        if img_w <= 0.0 || img_h <= 0.0 {
            continue;
        }

        let img_ratio = img_w / img_h;
        let win_ratio = win_w / win_h;

        let (draw_w, draw_h) = if win_ratio > img_ratio {
            (win_w, win_w / img_ratio)
        } else {
            (win_h * img_ratio, win_h)
        };

        node.width = Val::Px(draw_w);
        node.height = Val::Px(draw_h);
        node.left = Val::Px((win_w - draw_w) * 0.5);
        node.top = Val::Px((win_h - draw_h) * 0.5);
    }
}

pub fn handle_main_menu_buttons(
    mut interactions: Query<
        (&Interaction, &mut BackgroundColor, &MainMenuAction),
        Changed<Interaction>,
    >,
    mut next_state: ResMut<NextState<GameState>>,
    mut exit_writer: MessageWriter<AppExit>,
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    settings: Res<GameSettings>,
) {
    for (interaction, mut bg, action) in interactions.iter_mut() {
        match *interaction {
            Interaction::Pressed => {
                bg.0 = skin::button_pressed();
                match action {
                    MainMenuAction::Start => next_state.set(GameState::InGame),
                    MainMenuAction::Save => crate::ui::save::open_save_panel(
                        &mut commands,
                        &asset_server,
                        settings.language,
                    ),
                    MainMenuAction::Settings => {
                        crate::ui::settings::open_settings_panel(&mut commands)
                    }
                    MainMenuAction::Exit => {
                        let _ = exit_writer.write(AppExit::Success);
                    }
                }
            }
            Interaction::Hovered => bg.0 = skin::button_hover(),
            Interaction::None => bg.0 = skin::button_idle(),
        }
    }
}

fn spawn_button(
    parent: &mut ChildSpawnerCommands<'_>,
    asset_server: &AssetServer,
    font: &Handle<Font>,
    label: &str,
    action: MainMenuAction,
) {
    parent
        .spawn((
            Button,
            Node {
                width: Val::Percent(100.0),
                height: Val::Px(52.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(skin::button_idle()),
            ImageNode::new(skin::button_large(asset_server)),
            action,
        ))
        .with_children(|button| {
            button.spawn((
                Text::new(label.to_string()),
                TextFont {
                    font: font.clone(),
                    font_size: 28.0,
                    ..default()
                },
                TextColor(skin::text_primary()),
            ));
        });
}
