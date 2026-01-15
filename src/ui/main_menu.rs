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
    commands.spawn((
        MainMenuBackground,
        bg_sprite,
        Transform::from_xyz(0.0, 0.0, -100.0),
    ));

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
            spawn_button(parent, &font, "开始游戏", MainMenuAction::Start);
            spawn_button(parent, &font, "存档", MainMenuAction::Save);
            spawn_button(parent, &font, "设置", MainMenuAction::Settings);
            spawn_button(parent, &font, "退出", MainMenuAction::Exit);
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
    asset_server: Res<AssetServer>,
) {
    for (interaction, mut bg, action) in interactions.iter_mut() {
        match *interaction {
            Interaction::Pressed => {
                bg.0 = Color::srgb(0.8, 0.8, 1.0);
                match action {
                    MainMenuAction::Start => next_state.set(GameState::InGame),
                    MainMenuAction::Save => crate::ui::save::open_save_panel(&mut commands, &asset_server),
                    MainMenuAction::Settings => crate::ui::settings::open_settings_panel(&mut commands),
                    MainMenuAction::Exit => { let _ = exit_writer.write(AppExit::Success); }
                }
            }
            Interaction::Hovered => bg.0 = Color::srgb(0.6, 0.6, 0.8),
            Interaction::None => bg.0 = Color::srgb(0.25, 0.25, 0.35),
        }
    }
}

fn spawn_button(
    parent: &mut ChildSpawnerCommands<'_>,
    font: &Handle<Font>,
    label: &str,
    action: MainMenuAction,
) {
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
            BackgroundColor(Color::srgb(0.25, 0.25, 0.35)),
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
                TextColor(Color::WHITE),
            ));
        });
}
