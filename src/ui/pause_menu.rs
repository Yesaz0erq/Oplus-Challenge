use bevy::prelude::*;
use bevy::ui::Val;

use crate::state::GameState;
use crate::ui::main_menu::MainMenuAction;

#[derive(Component)]
pub struct PauseMenuUI;

pub fn spawn_pause_menu(mut commands: Commands, asset_server: Res<AssetServer>) {
    let font = asset_server.load("fonts/YuFanLixing.otf");

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
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.6)),
        ))
        .with_children(|parent| {
            spawn_button(
                parent,
                &font,
                "继续游戏",
                Color::srgb(0.3, 0.5, 0.9),
                MainMenuAction::Start,
            );
            spawn_button(
                parent,
                &font,
                "存档",
                Color::srgb(0.5, 0.4, 0.8),
                MainMenuAction::Save,
            );
            spawn_button(
                parent,
                &font,
                "设置",
                Color::srgb(0.4, 0.7, 0.4),
                MainMenuAction::Settings,
            );
            spawn_button(
                parent,
                &font,
                "返回主菜单",
                Color::srgb(0.8, 0.3, 0.3),
                MainMenuAction::Exit,
            );
        });
}

pub fn cleanup_pause_menu(mut commands: Commands, q: Query<Entity, With<PauseMenuUI>>) {
    if let Ok(e) = q.single() {
        commands.entity(e).try_despawn();
    }
}

pub fn handle_pause_menu_buttons(
    mut interactions: Query<
        (&Interaction, &mut BackgroundColor, &MainMenuAction),
        Changed<Interaction>,
    >,
    mut next_state: ResMut<NextState<GameState>>,
    mut commands: Commands,
    asset_server: Res<AssetServer>,
) {
    for (interaction, mut bg, action) in &mut interactions {
        match *interaction {
            Interaction::Pressed => {
                bg.0 = Color::srgb(0.8, 0.8, 1.0);
                match action {
                    MainMenuAction::Start => next_state.set(GameState::InGame),
                    MainMenuAction::Save => {
                        crate::ui::save::open_save_panel(&mut commands, &asset_server)
                    }
                    MainMenuAction::Settings => {
                        crate::ui::settings::open_settings_panel(&mut commands)
                    }
                    MainMenuAction::Exit => next_state.set(GameState::MainMenu),
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
    color: Color,
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
            BackgroundColor(color),
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
