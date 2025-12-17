use bevy::prelude::*;
use bevy::ui::Val;

use crate::state::GameState;

#[derive(Component)]
pub struct MainMenuUI;

#[derive(Component)]
pub struct MainMenuBackground;

#[derive(Component, Clone, Copy)]
pub enum MainMenuAction {
    Start,
    Save,
    Settings,
    Exit,
}

pub fn spawn_main_menu(mut commands: Commands, asset_server: Res<AssetServer>) {
    let font = asset_server.load("fonts/YuFanLixing.otf");

    let bg_handle: Handle<Image> = asset_server.load("main_background.png");
    let mut bg_sprite = Sprite::from_image(bg_handle);
    bg_sprite.custom_size = Some(Vec2::new(1920.0, 1080.0));
    commands.spawn((MainMenuBackground, bg_sprite, Transform::from_xyz(0.0, 0.0, -100.0)));

    commands
        .spawn((
            MainMenuUI,
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
            // Start
            parent
                .spawn((
                    Button,
                    Node {
                        width: Val::Px(200.0),
                        height: Val::Px(50.0),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        ..default()
                    },
                    BackgroundColor(Color::srgb(0.3, 0.5, 0.9)),
                    MainMenuAction::Start,
                ))
                .with_children(|button| {
                    button.spawn((
                        Text::new("开始游戏".to_string()),
                        TextFont {
                            font: font.clone(),
                            font_size: 28.0,
                            ..default()
                        },
                        TextColor(Color::WHITE),
                    ));
                });

            // Save
            parent
                .spawn((
                    Button,
                    Node {
                        width: Val::Px(200.0),
                        height: Val::Px(50.0),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        ..default()
                    },
                    BackgroundColor(Color::srgb(0.5, 0.4, 0.8)),
                    MainMenuAction::Save,
                ))
                .with_children(|button| {
                    button.spawn((
                        Text::new("存档".to_string()),
                        TextFont {
                            font: font.clone(),
                            font_size: 28.0,
                            ..default()
                        },
                        TextColor(Color::WHITE),
                    ));
                });

            // Settings
            parent
                .spawn((
                    Button,
                    Node {
                        width: Val::Px(200.0),
                        height: Val::Px(50.0),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        ..default()
                    },
                    BackgroundColor(Color::srgb(0.4, 0.7, 0.4)),
                    MainMenuAction::Settings,
                ))
                .with_children(|button| {
                    button.spawn((
                        Text::new("设置".to_string()),
                        TextFont {
                            font: font.clone(),
                            font_size: 28.0,
                            ..default()
                        },
                        TextColor(Color::WHITE),
                    ));
                });

            // Exit
            parent
                .spawn((
                    Button,
                    Node {
                        width: Val::Px(200.0),
                        height: Val::Px(50.0),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        ..default()
                    },
                    BackgroundColor(Color::srgb(0.8, 0.3, 0.3)),
                    MainMenuAction::Exit,
                ))
                .with_children(|button| {
                    button.spawn((
                        Text::new("退出".to_string()),
                        TextFont {
                            font,
                            font_size: 28.0,
                            ..default()
                        },
                        TextColor(Color::WHITE),
                    ));
                });
        });
}

pub fn cleanup_main_menu(
    mut commands: Commands,
    q_ui: Query<Entity, With<MainMenuUI>>,
    q_bg: Query<Entity, With<MainMenuBackground>>,
) {
    if let Ok(e) = q_ui.single() {
        commands.entity(e).try_despawn();
    }
    if let Ok(e) = q_bg.single() {
        commands.entity(e).try_despawn();
    }
}

pub fn handle_main_menu_buttons(
    mut interactions: Query<(&Interaction, &mut BackgroundColor, &MainMenuAction), Changed<Interaction>>,
    mut next_state: ResMut<NextState<GameState>>,
    mut exit_writer: MessageWriter<AppExit>,
    mut commands: Commands,
) {
    for (interaction, mut bg, action) in &mut interactions {
        match *interaction {
            Interaction::Pressed => {
                bg.0 = Color::srgb(0.8, 0.8, 1.0);
                match action {
                    MainMenuAction::Start => {
                        next_state.set(GameState::InGame);
                    }
                    MainMenuAction::Save => {
                        crate::ui::save::open_save_panel(&mut commands);
                    }
                    MainMenuAction::Settings => {
                        crate::ui::settings::open_settings_panel(&mut commands);
                    }
                    MainMenuAction::Exit => {
                        // 发送退出消息
                        exit_writer.write(AppExit::Success);
                    }
                }
            }
            Interaction::Hovered => {
                bg.0 = Color::srgb(0.6, 0.6, 0.8);
            }
            Interaction::None => {
                bg.0 = Color::srgb(0.25, 0.25, 0.35);
            }
        }
    }
}