use bevy::prelude::*;

use crate::state::GameState;

/// Game Over UI 插件
pub struct GameOverUiPlugin;

#[derive(Component)]
pub struct GameOverRoot;

/// 按钮类型
#[derive(Component)]
pub enum GameOverButton {
    Retry,
    MainMenu,
}

impl Plugin for GameOverUiPlugin {
    fn build(&self, app: &mut App) {
        app
            // 进入 GameOver 时生成 UI
            .add_systems(OnEnter(GameState::GameOver), setup_game_over_ui)
            // 离开 GameOver 时清理 UI
            .add_systems(OnExit(GameState::GameOver), cleanup_game_over_ui)
            // 在 GameOver 状态下处理按钮点击
            .add_systems(
                Update,
                handle_game_over_buttons.run_if(in_state(GameState::GameOver)),
            );
    }
}

/// 生成 Game Over 界面
fn setup_game_over_ui(mut commands: Commands, asset_server: Res<AssetServer>) {
    let font: Handle<Font> = asset_server.load("fonts/YuFanLixing.otf");

    // 半透明黑色全屏背景
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
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.6)),
            GameOverRoot,
        ))
        .with_children(|parent| {
            // 中间的面板
            parent
                .spawn((
                    Node {
                        flex_direction: FlexDirection::Column,
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        padding: UiRect::all(Val::Px(24.0)),
                        row_gap: Val::Px(16.0),
                        ..default()
                    },
                    BackgroundColor(Color::srgba(0.1, 0.1, 0.2, 0.9)),
                ))
                .with_children(|parent| {
                    // 标题：Game Over
                    parent.spawn((
                        Text::new("Game Over"),
                        TextFont {
                            font: font.clone(),
                            font_size: 36.0,
                            ..default()
                        },
                        TextColor(Color::WHITE),
                    ));

                    // 按钮容器（横向排布）
                    parent
                        .spawn((
                            Node {
                                flex_direction: FlexDirection::Row,
                                column_gap: Val::Px(16.0),
                                margin: UiRect::top(Val::Px(16.0)),
                                ..default()
                            },
                        ))
                        .with_children(|parent| {
                            // Retry 按钮
                            parent
                                .spawn((
                                    Button,
                                    Node {
                                        padding: UiRect::all(Val::Px(10.0)),
                                        justify_content: JustifyContent::Center,
                                        align_items: AlignItems::Center,
                                        ..default()
                                    },
                                    BackgroundColor(Color::srgba(0.2, 0.2, 0.4, 1.0)),
                                    GameOverButton::Retry,
                                ))
                                .with_children(|button| {
                                    button.spawn((
                                        Text::new("Retry"),
                                        TextFont {
                                            font: font.clone(),
                                            font_size: 20.0,
                                            ..default()
                                        },
                                        TextColor(Color::WHITE),
                                    ));
                                });

                            // Main Menu 按钮
                            parent
                                .spawn((
                                    Button,
                                    Node {
                                        padding: UiRect::all(Val::Px(10.0)),
                                        justify_content: JustifyContent::Center,
                                        align_items: AlignItems::Center,
                                        ..default()
                                    },
                                    BackgroundColor(Color::srgba(0.2, 0.2, 0.4, 1.0)),
                                    GameOverButton::MainMenu,
                                ))
                                .with_children(|button| {
                                    button.spawn((
                                        Text::new("Main Menu"),
                                        TextFont {
                                            font: font.clone(),
                                            font_size: 20.0,
                                            ..default()
                                        },
                                        TextColor(Color::WHITE),
                                    ));
                                });
                        });
                });
        });
}

/// 处理 Game Over 按钮点击：
/// - Retry：重新进入 InGame
/// - Main Menu：切回主菜单状态 GameState::MainMenu
fn handle_game_over_buttons(
    mut next_state: ResMut<NextState<GameState>>,
    mut q: Query<(&Interaction, &GameOverButton), (Changed<Interaction>, With<Button>)>,
) {
    for (interaction, button) in &mut q {
        if *interaction != Interaction::Pressed {
            continue;
        }

        match button {
            GameOverButton::Retry => {
                // 你可以视情况在别的地方重置玩家和场景
                next_state.set(GameState::InGame);
            }
            GameOverButton::MainMenu => {
                next_state.set(GameState::MainMenu);
            }
        }
    }
}

/// 递归删除 UI 根节点及子节点
fn despawn_with_children(
    commands: &mut Commands,
    children_q: &Query<&Children>,
    entity: Entity,
) {
    if let Ok(children) = children_q.get(entity) {
        // 注意这里用 `for &child in ...`，不要再解引用一次
        for child in children.iter() {
            despawn_with_children(commands, children_q, child);
        }
    }
    commands.entity(entity).despawn();
}

/// 清理 Game Over UI
fn cleanup_game_over_ui(
    mut commands: Commands,
    roots: Query<Entity, With<GameOverRoot>>,
    children_q: Query<&Children>,
) {
    for root in &roots {
        despawn_with_children(&mut commands, &children_q, root);
    }
}
