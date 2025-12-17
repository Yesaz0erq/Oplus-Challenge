use bevy::prelude::*;
use bevy::ui::Val;

use crate::save::{refresh_save_slots_from_disk, CurrentSlot, LoadSlotEvent, PendingLoad, SaveSlots};
use crate::state::GameState;

use crate::enemy::Enemy;

/// Game Over UI 插件
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
            (
                handle_game_over_buttons,
                handle_manual_save_slot_buttons,
            )
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
    // 清掉敌人（失败后必须完全重置）
    for e in &enemies {
        commands.entity(e).despawn();
    }

    // 清空读档/当前槽，防止“重新开始 = 继续当前 autosave”
    pending.file_name = None;
    current.file_name = None;

    // 刷新存档列表（从 ./saves 扫描）
    refresh_save_slots_from_disk(&mut slots);
}

fn setup_game_over_ui(mut commands: Commands, asset_server: Res<AssetServer>, slots: Res<SaveSlots>) {
    let font: Handle<Font> = asset_server.load("fonts/YuFanLixing.otf");

    // 只显示手动存档
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
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.65)),
            GameOverRoot,
        ))
        .with_children(|parent| {
            // 中央面板（尽量沿用你现有 UI 的深色卡片风格）
            parent
                .spawn((
                    Node {
                        width: Val::Px(720.0),
                        padding: UiRect::all(Val::Px(26.0)),
                        flex_direction: FlexDirection::Column,
                        row_gap: Val::Px(14.0),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        ..default()
                    },
                    BackgroundColor(Color::srgba(0.12, 0.12, 0.16, 0.95)),
                ))
                .with_children(|panel| {
                    panel.spawn((
                        Text::new("游戏失败"),
                        TextFont {
                            font: font.clone(),
                            font_size: 40.0,
                            ..default()
                        },
                        TextColor(Color::WHITE),
                    ));

                    panel.spawn((
                        Text::new("请选择一个【手动存档】重新开始（不会使用自动存档）"),
                        TextFont {
                            font: font.clone(),
                            font_size: 18.0,
                            ..default()
                        },
                        TextColor(Color::srgba(1.0, 1.0, 1.0, 0.85)),
                    ));

                    // 存档列表容器
                    panel
                        .spawn((
                            Node {
                                width: Val::Px(640.0),
                                flex_direction: FlexDirection::Column,
                                row_gap: Val::Px(10.0),
                                margin: UiRect::top(Val::Px(10.0)),
                                ..default()
                            },
                        ))
                        .with_children(|list| {
                            if manual_slots.is_empty() {
                                list.spawn((
                                    Text::new("暂无手动存档：请先在游戏内打开“存档面板”进行手动保存。"),
                                    TextFont {
                                        font: font.clone(),
                                        font_size: 18.0,
                                        ..default()
                                    },
                                    TextColor(Color::srgba(1.0, 1.0, 1.0, 0.8)),
                                ));
                            } else {
                                for s in manual_slots {
                                    list.spawn((
                                        Button,
                                        Node {
                                            width: Val::Px(640.0),
                                            height: Val::Px(44.0),
                                            padding: UiRect::horizontal(Val::Px(14.0)),
                                            justify_content: JustifyContent::SpaceBetween,
                                            align_items: AlignItems::Center,
                                            ..default()
                                        },
                                        BackgroundColor(Color::srgb(0.25, 0.25, 0.35)),
                                        ManualSaveSlotButton {
                                            file_name: s.file_name.clone(),
                                        },
                                    ))
                                    .with_children(|btn| {
                                        btn.spawn((
                                            Text::new(format!("存档：{}", s.display_name)),
                                            TextFont {
                                                font: font.clone(),
                                                font_size: 18.0,
                                                ..default()
                                            },
                                            TextColor(Color::WHITE),
                                        ));
                                        btn.spawn((
                                            Text::new("加载并重新开始"),
                                            TextFont {
                                                font: font.clone(),
                                                font_size: 16.0,
                                                ..default()
                                            },
                                            TextColor(Color::srgba(1.0, 1.0, 1.0, 0.85)),
                                        ));
                                    });
                                }
                            }
                        });

                    // 底部按钮：返回主菜单
                    panel
                        .spawn((
                            Node {
                                flex_direction: FlexDirection::Row,
                                column_gap: Val::Px(14.0),
                                margin: UiRect::top(Val::Px(16.0)),
                                ..default()
                            },
                        ))
                        .with_children(|row| {
                            row.spawn((
                                Button,
                                Node {
                                    width: Val::Px(220.0),
                                    height: Val::Px(46.0),
                                    justify_content: JustifyContent::Center,
                                    align_items: AlignItems::Center,
                                    ..default()
                                },
                                BackgroundColor(Color::srgb(0.20, 0.20, 0.40)),
                                GameOverButton::BackToMainMenu,
                            ))
                            .with_children(|btn| {
                                btn.spawn((
                                    Text::new("返回标题界面"),
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
    for (interaction, mut bg, btn) in &mut q {
        match *interaction {
            Interaction::Pressed => {
                bg.0 = Color::srgb(0.8, 0.8, 1.0);

                for e in &enemies {
                    commands.entity(e).despawn();
                }

                load_tx.write(LoadSlotEvent {
                    file_name: btn.file_name.clone(),
                });
                next_state.set(GameState::InGame);
            }
            Interaction::Hovered => bg.0 = Color::srgb(0.6, 0.6, 0.8),
            Interaction::None => bg.0 = Color::srgb(0.25, 0.25, 0.35),
        }
    }
}

fn handle_game_over_buttons(
    mut next_state: ResMut<NextState<GameState>>,
    mut q: Query<(&Interaction, &GameOverButton), (Changed<Interaction>, With<Button>)>,
) {
    for (interaction, button) in &mut q {
        if *interaction != Interaction::Pressed {
            continue;
        }
        match button {
            GameOverButton::BackToMainMenu => next_state.set(GameState::MainMenu),
        }
    }
}

fn despawn_with_children(commands: &mut Commands, children_q: &Query<&Children>, entity: Entity) {
    if let Ok(children) = children_q.get(entity) {
        for child in children.iter() {
            despawn_with_children(commands, children_q, child);
        }
    }
    commands.entity(entity).despawn();
}

fn cleanup_game_over_ui(
    mut commands: Commands,
    roots: Query<Entity, With<GameOverRoot>>,
    children_q: Query<&Children>,
) {
    for root in &roots {
        despawn_with_children(&mut commands, &children_q, root);
    }
}
