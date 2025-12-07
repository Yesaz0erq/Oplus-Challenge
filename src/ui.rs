use bevy::app::AppExit;
use bevy::audio::{GlobalVolume, Volume};
use bevy::input::ButtonInput;
use bevy::prelude::*;
use bevy::ui::{UiRect, Val};
use bevy::window::{MonitorSelection, PrimaryWindow, Window, WindowMode, WindowResolution};

use crate::save::{
    CurrentSlot, ManualSaveEvent, SaveSlots, ensure_slot_in_list, generate_slot_display_name,
    refresh_save_slots_from_disk,
};
use crate::state::GameState;

/// 预设分辨率列表（按需修改）
const RESOLUTIONS: &[(u32, u32)] = &[(1280, 720), (1600, 900), (1920, 1080)];

/// 全局游戏设置（分辨率索引 + 音量 + 全屏）
#[derive(Resource)]
pub struct GameSettings {
    pub resolution_index: usize,
    /// 0.0 ~ 1.0
    pub volume: f32,
    pub fullscreen: bool,
}

impl Default for GameSettings {
    fn default() -> Self {
        Self {
            resolution_index: 0,
            volume: 0.8,
            fullscreen: false,
        }
    }
}

pub struct MenuPlugin;

#[derive(Component)]
struct MainMenuUI;

#[derive(Component)]
struct PauseMenuUI;

#[derive(Component)]
struct SettingsPanel;

#[derive(Component)]
struct SavePanel;

/// 设置面板里的“当前分辨率”文字
#[derive(Component)]
struct ResolutionText;

/// 设置面板里的“当前音量”文字
#[derive(Component)]
struct VolumeText;

/// 设置面板里的“当前显示模式”文字（全屏 / 窗口）
#[derive(Component)]
struct FullscreenText;

#[derive(Component, Clone, Copy)]
enum MainMenuAction {
    Start,
    Save,
    Settings,
    Exit,
}

#[derive(Component, Clone, Copy)]
enum PauseMenuAction {
    Resume,
    Save,
    Settings,
    Exit,
}

#[derive(Component, Clone, Copy)]
enum SavePanelAction {
    Save,
    Close,
}

#[derive(Component)]
struct SaveSlotButton {
    file_name: String,
}

/// 设置面板按钮的行为
#[derive(Component, Clone, Copy)]
enum SettingsButtonAction {
    ResolutionDown,
    ResolutionUp,
    VolumeDown,
    VolumeUp,
    ToggleFullscreen,
    ClosePanel,
}

impl Plugin for MenuPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<GameSettings>()
            .add_systems(OnEnter(GameState::MainMenu), spawn_main_menu)
            .add_systems(OnExit(GameState::MainMenu), cleanup_main_menu)
            .add_systems(OnEnter(GameState::Paused), spawn_pause_menu)
            .add_systems(OnExit(GameState::Paused), cleanup_pause_menu)
            .add_systems(
                Update,
                (
                    handle_main_menu_buttons.run_if(in_state(GameState::MainMenu)),
                    handle_pause_menu_buttons.run_if(in_state(GameState::Paused)),
                    handle_settings_buttons,
                    handle_save_panel_button_interaction,
                    handle_save_panel_layout,
                    close_settings_on_esc,
                    close_save_panel_on_esc,
                ),
            );
    }
}

/// 主菜单 UI
fn spawn_main_menu(mut commands: Commands, asset_server: Res<AssetServer>) {
    let font = asset_server.load("fonts/YuFanLixing.otf");

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
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.6)),
        ))
        .with_children(|parent| {
            // “开始游戏”
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

            // “存档”
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

            // “设置”
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

            // “退出”
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

/// 暂停菜单 UI
fn spawn_pause_menu(mut commands: Commands, asset_server: Res<AssetServer>) {
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
            // “继续游戏”
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
                    PauseMenuAction::Resume,
                ))
                .with_children(|button| {
                    button.spawn((
                        Text::new("继续游戏".to_string()),
                        TextFont {
                            font: font.clone(),
                            font_size: 28.0,
                            ..default()
                        },
                        TextColor(Color::WHITE),
                    ));
                });

            // “存档”
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
                    PauseMenuAction::Save,
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

            // “设置”
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
                    PauseMenuAction::Settings,
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

            // “退出”
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
                    PauseMenuAction::Exit,
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

/// 主菜单按钮交互
fn handle_main_menu_buttons(
    mut commands: Commands,
    mut interactions: Query<
        (&Interaction, &mut BackgroundColor, &MainMenuAction),
        Changed<Interaction>,
    >,
    mut next_state: ResMut<NextState<GameState>>,
    mut exit_writer: MessageWriter<AppExit>,
    asset_server: Res<AssetServer>,
    settings: Res<GameSettings>,
    settings_panel: Query<Entity, With<SettingsPanel>>,
    save_panel: Query<Entity, With<SavePanel>>,
    mut slots: ResMut<SaveSlots>,
    current: Res<CurrentSlot>,
) {
    for (interaction, mut color, action) in &mut interactions {
        match *interaction {
            Interaction::Pressed => match action {
                MainMenuAction::Start => next_state.set(GameState::InGame),
                MainMenuAction::Save => {
                    ensure_save_panel(
                        &mut commands,
                        &save_panel,
                        &asset_server,
                        &mut slots,
                        &current,
                    );
                }
                MainMenuAction::Settings => {
                    let (w, h) = RESOLUTIONS[settings.resolution_index];
                    ensure_settings_panel(
                        &mut commands,
                        &settings_panel,
                        &asset_server,
                        (w, h),
                        settings.volume,
                        settings.fullscreen,
                    );
                }
                MainMenuAction::Exit => {
                    exit_writer.write(AppExit::Success);
                }
            },
            Interaction::Hovered => {
                color.0 = Color::srgb(0.7, 0.7, 0.9);
            }
            Interaction::None => {
                color.0 = Color::srgb(0.25, 0.25, 0.35);
            }
        }
    }
}

/// 暂停菜单按钮交互
fn handle_pause_menu_buttons(
    mut commands: Commands,
    mut interactions: Query<
        (&Interaction, &mut BackgroundColor, &PauseMenuAction),
        Changed<Interaction>,
    >,
    mut next_state: ResMut<NextState<GameState>>,
    mut exit_writer: MessageWriter<AppExit>,
    asset_server: Res<AssetServer>,
    settings: Res<GameSettings>,
    settings_panel: Query<Entity, With<SettingsPanel>>,
    save_panel: Query<Entity, With<SavePanel>>,
    mut slots: ResMut<SaveSlots>,
    current: Res<CurrentSlot>,
) {
    for (interaction, mut color, action) in &mut interactions {
        match *interaction {
            Interaction::Pressed => match action {
                PauseMenuAction::Resume => next_state.set(GameState::InGame),
                PauseMenuAction::Save => {
                    ensure_save_panel(
                        &mut commands,
                        &save_panel,
                        &asset_server,
                        &mut slots,
                        &current,
                    );
                }
                PauseMenuAction::Settings => {
                    let (w, h) = RESOLUTIONS[settings.resolution_index];
                    ensure_settings_panel(
                        &mut commands,
                        &settings_panel,
                        &asset_server,
                        (w, h),
                        settings.volume,
                        settings.fullscreen,
                    );
                }
                PauseMenuAction::Exit => {
                    exit_writer.write(AppExit::Success);
                }
            },
            Interaction::Hovered => {
                color.0 = Color::srgb(0.7, 0.7, 0.9);
            }
            Interaction::None => {
                color.0 = Color::srgb(0.25, 0.25, 0.35);
            }
        }
    }
}

/// 只在还没创建设置面板时生成一份，并根据当前设置填充文本
fn ensure_settings_panel(
    commands: &mut Commands,
    panel_query: &Query<Entity, With<SettingsPanel>>,
    asset_server: &AssetServer,
    resolution: (u32, u32),
    volume: f32,
    fullscreen: bool,
) {
    if !panel_query.is_empty() {
        return;
    }

    let font = asset_server.load("fonts/YuFanLixing.otf");
    let (w, h) = resolution;
    let volume_percent = (volume * 100.0).round();
    let fullscreen_str = if fullscreen { "全屏" } else { "窗口" };

    commands
        .spawn((
            SettingsPanel,
            Node {
                width: Val::Percent(60.0),
                height: Val::Px(200.0),
                position_type: PositionType::Absolute,
                left: Val::Percent(20.0),
                top: Val::Percent(20.0),
                justify_content: JustifyContent::FlexStart,
                align_items: AlignItems::Stretch,
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(8.0),
                padding: UiRect::all(Val::Px(12.0)),
                ..default()
            },
            BackgroundColor(Color::srgba(0.1, 0.1, 0.1, 0.85)),
        ))
        .with_children(|parent| {
            // 标题
            parent.spawn((
                Text::new("设置".to_string()),
                TextFont {
                    font: font.clone(),
                    font_size: 24.0,
                    ..default()
                },
                TextColor(Color::WHITE),
            ));

            // 分辨率行：左 label，右 [- 数值 +]
            parent
                .spawn((Node {
                    width: Val::Percent(100.0),
                    height: Val::Auto,
                    flex_direction: FlexDirection::Row,
                    justify_content: JustifyContent::SpaceBetween,
                    align_items: AlignItems::Center,
                    ..default()
                },))
                .with_children(|row| {
                    // 左侧 label
                    row.spawn((
                        Text::new("分辨率".to_string()),
                        TextFont {
                            font: font.clone(),
                            font_size: 20.0,
                            ..default()
                        },
                        TextColor(Color::WHITE),
                    ));

                    // 右侧 [- 数值 +] 一组
                    row.spawn((Node {
                        flex_direction: FlexDirection::Row,
                        align_items: AlignItems::Center,
                        column_gap: Val::Px(4.0),
                        ..default()
                    },))
                        .with_children(|right| {
                            // -
                            right
                                .spawn((
                                    Button,
                                    Node {
                                        width: Val::Px(24.0),
                                        height: Val::Px(24.0),
                                        justify_content: JustifyContent::Center,
                                        align_items: AlignItems::Center,
                                        ..default()
                                    },
                                    BackgroundColor(Color::srgb(0.3, 0.3, 0.3)),
                                    SettingsButtonAction::ResolutionDown,
                                ))
                                .with_children(|b| {
                                    b.spawn((
                                        Text::new("-".to_string()),
                                        TextFont {
                                            font: font.clone(),
                                            font_size: 18.0,
                                            ..default()
                                        },
                                        TextColor(Color::WHITE),
                                    ));
                                });

                            // 数值
                            right.spawn((
                                Text::new(format!("{w} x {h}")),
                                TextFont {
                                    font: font.clone(),
                                    font_size: 20.0,
                                    ..default()
                                },
                                TextColor(Color::srgb(0.9, 0.9, 0.3)),
                                ResolutionText,
                            ));

                            // +
                            right
                                .spawn((
                                    Button,
                                    Node {
                                        width: Val::Px(24.0),
                                        height: Val::Px(24.0),
                                        justify_content: JustifyContent::Center,
                                        align_items: AlignItems::Center,
                                        ..default()
                                    },
                                    BackgroundColor(Color::srgb(0.3, 0.3, 0.3)),
                                    SettingsButtonAction::ResolutionUp,
                                ))
                                .with_children(|b| {
                                    b.spawn((
                                        Text::new("+".to_string()),
                                        TextFont {
                                            font: font.clone(),
                                            font_size: 18.0,
                                            ..default()
                                        },
                                        TextColor(Color::WHITE),
                                    ));
                                });
                        });
                });

            // 音量行：左 label，右 [- 数值 +]
            parent
                .spawn((Node {
                    width: Val::Percent(100.0),
                    height: Val::Auto,
                    flex_direction: FlexDirection::Row,
                    justify_content: JustifyContent::SpaceBetween,
                    align_items: AlignItems::Center,
                    ..default()
                },))
                .with_children(|row| {
                    // 左侧 label
                    row.spawn((
                        Text::new("音量".to_string()),
                        TextFont {
                            font: font.clone(),
                            font_size: 20.0,
                            ..default()
                        },
                        TextColor(Color::WHITE),
                    ));

                    // 右侧 [- 数值 +]
                    row.spawn((Node {
                        flex_direction: FlexDirection::Row,
                        align_items: AlignItems::Center,
                        column_gap: Val::Px(4.0),
                        ..default()
                    },))
                        .with_children(|right| {
                            // -
                            right
                                .spawn((
                                    Button,
                                    Node {
                                        width: Val::Px(24.0),
                                        height: Val::Px(24.0),
                                        justify_content: JustifyContent::Center,
                                        align_items: AlignItems::Center,
                                        ..default()
                                    },
                                    BackgroundColor(Color::srgb(0.3, 0.3, 0.3)),
                                    SettingsButtonAction::VolumeDown,
                                ))
                                .with_children(|b| {
                                    b.spawn((
                                        Text::new("-".to_string()),
                                        TextFont {
                                            font: font.clone(),
                                            font_size: 18.0,
                                            ..default()
                                        },
                                        TextColor(Color::WHITE),
                                    ));
                                });

                            // 数值
                            right.spawn((
                                Text::new(format!("{volume_percent:.0} %")),
                                TextFont {
                                    font: font.clone(),
                                    font_size: 20.0,
                                    ..default()
                                },
                                TextColor(Color::srgb(0.9, 0.9, 0.3)),
                                VolumeText,
                            ));

                            // +
                            right
                                .spawn((
                                    Button,
                                    Node {
                                        width: Val::Px(24.0),
                                        height: Val::Px(24.0),
                                        justify_content: JustifyContent::Center,
                                        align_items: AlignItems::Center,
                                        ..default()
                                    },
                                    BackgroundColor(Color::srgb(0.3, 0.3, 0.3)),
                                    SettingsButtonAction::VolumeUp,
                                ))
                                .with_children(|b| {
                                    b.spawn((
                                        Text::new("+".to_string()),
                                        TextFont {
                                            font: font.clone(),
                                            font_size: 18.0,
                                            ..default()
                                        },
                                        TextColor(Color::WHITE),
                                    ));
                                });
                        });
                });

            // 显示模式行：左 label，右 [状态 + 切换按钮]
            parent
                .spawn((Node {
                    width: Val::Percent(100.0),
                    height: Val::Auto,
                    flex_direction: FlexDirection::Row,
                    justify_content: JustifyContent::SpaceBetween,
                    align_items: AlignItems::Center,
                    ..default()
                },))
                .with_children(|row| {
                    // 左侧 label
                    row.spawn((
                        Text::new("显示模式".to_string()),
                        TextFont {
                            font: font.clone(),
                            font_size: 20.0,
                            ..default()
                        },
                        TextColor(Color::WHITE),
                    ));

                    // 右侧 [状态 + 按钮]
                    row.spawn((Node {
                        flex_direction: FlexDirection::Row,
                        align_items: AlignItems::Center,
                        column_gap: Val::Px(8.0),
                        ..default()
                    },))
                        .with_children(|right| {
                            // 状态文字
                            right.spawn((
                                Text::new(fullscreen_str.to_string()),
                                TextFont {
                                    font: font.clone(),
                                    font_size: 20.0,
                                    ..default()
                                },
                                TextColor(Color::srgb(0.9, 0.9, 0.3)),
                                FullscreenText,
                            ));

                            // 切换按钮
                            right
                                .spawn((
                                    Button,
                                    Node {
                                        width: Val::Px(80.0),
                                        height: Val::Px(28.0),
                                        justify_content: JustifyContent::Center,
                                        align_items: AlignItems::Center,
                                        ..default()
                                    },
                                    BackgroundColor(Color::srgb(0.3, 0.3, 0.3)),
                                    SettingsButtonAction::ToggleFullscreen,
                                ))
                                .with_children(|b| {
                                    b.spawn((
                                        Text::new("切换".to_string()),
                                        TextFont {
                                            font: font.clone(),
                                            font_size: 18.0,
                                            ..default()
                                        },
                                        TextColor(Color::WHITE),
                                    ));
                                });
                        });
                });

            // 底部“关闭设置”按钮行
            parent
                .spawn((Node {
                    width: Val::Percent(100.0),
                    height: Val::Auto,
                    flex_direction: FlexDirection::Row,
                    justify_content: JustifyContent::FlexEnd,
                    align_items: AlignItems::Center,
                    ..default()
                },))
                .with_children(|row| {
                    row.spawn((
                        Button,
                        Node {
                            width: Val::Px(80.0),
                            height: Val::Px(30.0),
                            justify_content: JustifyContent::Center,
                            align_items: AlignItems::Center,
                            ..default()
                        },
                        BackgroundColor(Color::srgb(0.5, 0.3, 0.3)),
                        SettingsButtonAction::ClosePanel,
                    ))
                    .with_children(|b| {
                        b.spawn((
                            Text::new("关闭".to_string()),
                            TextFont {
                                font: font.clone(),
                                font_size: 18.0,
                                ..default()
                            },
                            TextColor(Color::WHITE),
                        ));
                    });
                });
        });
}

/// 处理设置面板按钮（分辨率 / 音量 / 全屏 / 关闭）
/// 使用 ParamSet 解决 B0001：避免同时对 Text 做多个 &mut Query
fn handle_settings_buttons(
    mut commands: Commands,
    mut interactions: Query<
        (&Interaction, &mut BackgroundColor, &SettingsButtonAction),
        Changed<Interaction>,
    >,
    mut settings: ResMut<GameSettings>,
    mut windows: Query<&mut Window, With<PrimaryWindow>>,
    mut global_volume: ResMut<GlobalVolume>,
    mut text_queries: ParamSet<(
        Query<&mut Text, With<ResolutionText>>,
        Query<&mut Text, With<VolumeText>>,
        Query<&mut Text, With<FullscreenText>>,
    )>,
    panel: Query<Entity, With<SettingsPanel>>,
    children: Query<&Children>,
) {
    for (interaction, mut color, action) in &mut interactions {
        match *interaction {
            Interaction::Pressed => {
                match action {
                    SettingsButtonAction::ClosePanel => {
                        // 关闭设置面板
                        despawn_all::<SettingsPanel>(&mut commands, &panel, &children);
                        continue;
                    }
                    SettingsButtonAction::ToggleFullscreen => {
                        if let Some(mut window) = windows.iter_mut().next() {
                            // 当前是否全屏：不是 Windowed 就当作全屏
                            let currently_fullscreen = !matches!(window.mode, WindowMode::Windowed);

                            if currently_fullscreen {
                                // 回到窗口模式，并应用窗口分辨率设置
                                window.mode = WindowMode::Windowed;
                                settings.fullscreen = false;

                                let (w, h) = RESOLUTIONS[settings.resolution_index];
                                window.resolution = WindowResolution::new(w, h);
                            } else {
                                // 进入无边框全屏，使用当前显示器的最大分辨率
                                window.mode =
                                    WindowMode::BorderlessFullscreen(MonitorSelection::Current);
                                settings.fullscreen = true;
                                // 不再使用手动的 window.resolution，交由全屏模式决定分辨率
                            }
                        }

                        // 更新显示模式文字
                        if let Some(mut text) = text_queries.p2().iter_mut().next() {
                            let s = if settings.fullscreen {
                                "全屏"
                            } else {
                                "窗口"
                            };
                            *text = Text::new(s.to_string());
                        }

                        color.0 = Color::srgb(0.7, 0.7, 0.9);
                    }
                    SettingsButtonAction::ResolutionDown | SettingsButtonAction::ResolutionUp => {
                        // 只有窗口模式才实际应用分辨率
                        match action {
                            SettingsButtonAction::ResolutionDown => {
                                if settings.resolution_index == 0 {
                                    settings.resolution_index = RESOLUTIONS.len() - 1;
                                } else {
                                    settings.resolution_index -= 1;
                                }
                            }
                            SettingsButtonAction::ResolutionUp => {
                                settings.resolution_index =
                                    (settings.resolution_index + 1) % RESOLUTIONS.len();
                            }
                            _ => {}
                        }

                        // 更新分辨率文本（即使在全屏，也可以看做“窗口模式下预设分辨率”）
                        if let Some(mut text) = text_queries.p0().iter_mut().next() {
                            let (w, h) = RESOLUTIONS[settings.resolution_index];
                            *text = Text::new(format!("{w} x {h}"));
                        }

                        // 只有在窗口化时才修改 window.resolution
                        if let Some(mut window) = windows.iter_mut().next() {
                            if matches!(window.mode, WindowMode::Windowed) {
                                let (w, h) = RESOLUTIONS[settings.resolution_index];
                                window.resolution = WindowResolution::new(w, h);
                            }
                        }

                        color.0 = Color::srgb(0.7, 0.7, 0.9);
                    }
                    SettingsButtonAction::VolumeDown | SettingsButtonAction::VolumeUp => {
                        // 只修改音量，不动分辨率 / 全屏
                        match action {
                            SettingsButtonAction::VolumeDown => {
                                settings.volume = (settings.volume - 0.1).max(0.0);
                            }
                            SettingsButtonAction::VolumeUp => {
                                settings.volume = (settings.volume + 0.1).min(1.0);
                            }
                            _ => {}
                        }

                        // 应用到全局音量
                        global_volume.volume = Volume::Linear(settings.volume);

                        // 更新音量文字
                        if let Some(mut text) = text_queries.p1().iter_mut().next() {
                            let percent = (settings.volume * 100.0).round();
                            *text = Text::new(format!("{percent:.0} %"));
                        }

                        // 这里不改 window.resolution，也不管 window.mode
                        color.0 = Color::srgb(0.7, 0.7, 0.9);
                    }
                }
            }
            Interaction::Hovered => {
                color.0 = Color::srgb(0.6, 0.6, 0.8);
            }
            Interaction::None => {
                color.0 = Color::srgb(0.3, 0.3, 0.3);
            }
        }
    }
}

fn ensure_save_panel(
    commands: &mut Commands,
    panel_query: &Query<Entity, With<SavePanel>>,
    asset_server: &AssetServer,
    slots: &mut SaveSlots,
    current: &CurrentSlot,
) {
    if !panel_query.is_empty() {
        return;
    }

    let font = asset_server.load("fonts/YuFanLixing.otf");

    refresh_save_slots_from_disk(slots);

    commands
        .spawn((
            SavePanel,
            Node {
                width: Val::Percent(60.0),
                height: Val::Percent(60.0),
                position_type: PositionType::Absolute,
                left: Val::Percent(20.0),
                top: Val::Percent(20.0),
                justify_content: JustifyContent::FlexStart,
                align_items: AlignItems::Stretch,
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(12.0),
                padding: UiRect::all(Val::Px(12.0)),
                ..default()
            },
            BackgroundColor(Color::srgba(0.12, 0.12, 0.12, 0.9)),
        ))
        .with_children(|parent| {
            parent.spawn((
                Text::new("存档".to_string()),
                TextFont {
                    font: font.clone(),
                    font_size: 24.0,
                    ..default()
                },
                TextColor(Color::WHITE),
            ));

            parent
                .spawn((
                    Node {
                        width: Val::Percent(100.0),
                        height: Val::Percent(100.0),
                        flex_direction: FlexDirection::Column,
                        row_gap: Val::Px(8.0),
                        ..default()
                    },
                    BackgroundColor(Color::srgba(0.08, 0.08, 0.08, 0.95)),
                ))
                .with_children(|list| {
                    if slots.slots.is_empty() {
                        list.spawn((
                            Text::new("暂无存档，点击“保存进度”创建新存档".to_string()),
                            TextFont {
                                font: font.clone(),
                                font_size: 18.0,
                                ..default()
                            },
                            TextColor(Color::srgb(0.8, 0.8, 0.8)),
                        ));
                    } else {
                        for slot in &slots.slots {
                            let selected = current
                                .file_name
                                .as_ref()
                                .map(|f| f == &slot.file_name)
                                .unwrap_or(false);
                            let base_color = if selected {
                                Color::srgb(0.3, 0.45, 0.8)
                            } else {
                                Color::srgb(0.25, 0.25, 0.25)
                            };

                            list.spawn((
                                Button,
                                Node {
                                    width: Val::Percent(100.0),
                                    height: Val::Px(40.0),
                                    justify_content: JustifyContent::FlexStart,
                                    align_items: AlignItems::Center,
                                    padding: UiRect::axes(Val::Px(12.0), Val::Px(0.0)),
                                    ..default()
                                },
                                BackgroundColor(base_color),
                                SaveSlotButton {
                                    file_name: slot.file_name.clone(),
                                },
                            ))
                            .with_children(|button| {
                                button.spawn((
                                    Text::new(slot.display_name.clone()),
                                    TextFont {
                                        font: font.clone(),
                                        font_size: 20.0,
                                        ..default()
                                    },
                                    TextColor(Color::WHITE),
                                ));
                            });
                        }
                    }
                });

            parent
                .spawn((Node {
                    width: Val::Percent(100.0),
                    height: Val::Auto,
                    flex_direction: FlexDirection::Row,
                    justify_content: JustifyContent::FlexEnd,
                    column_gap: Val::Px(12.0),
                    ..default()
                },))
                .with_children(|row| {
                    row.spawn((
                        Button,
                        Node {
                            width: Val::Px(140.0),
                            height: Val::Px(44.0),
                            justify_content: JustifyContent::Center,
                            align_items: AlignItems::Center,
                            ..default()
                        },
                        BackgroundColor(Color::srgb(0.35, 0.55, 0.9)),
                        SavePanelAction::Save,
                    ))
                    .with_children(|button| {
                        button.spawn((
                            Text::new("保存进度".to_string()),
                            TextFont {
                                font: font.clone(),
                                font_size: 20.0,
                                ..default()
                            },
                            TextColor(Color::WHITE),
                        ));
                    });

                    row.spawn((
                        Button,
                        Node {
                            width: Val::Px(120.0),
                            height: Val::Px(44.0),
                            justify_content: JustifyContent::Center,
                            align_items: AlignItems::Center,
                            ..default()
                        },
                        BackgroundColor(Color::srgb(0.5, 0.5, 0.5)),
                        SavePanelAction::Close,
                    ))
                    .with_children(|button| {
                        button.spawn((
                            Text::new("关闭".to_string()),
                            TextFont {
                                font,
                                font_size: 20.0,
                                ..default()
                            },
                            TextColor(Color::WHITE),
                        ));
                    });
                });
        });
}

// -------------------------
// System A: only handle button interaction (Interaction + BackgroundColor)
// -------------------------
fn handle_save_panel_button_interaction(
    mut interaction_q: Query<
        (
            &Interaction,
            &mut BackgroundColor,
            Option<&SaveSlotButton>,
            Option<&SavePanelAction>,
        ),
        (Changed<Interaction>, With<Button>),
    >,
    mut current: ResMut<CurrentSlot>,
    mut slots: ResMut<SaveSlots>,
    mut manual_save_writer: MessageWriter<ManualSaveEvent>,
    mut commands: Commands,
    panel_roots: Query<Entity, With<SavePanel>>,
    children: Query<&Children>,
) {
    for (interaction, mut color, slot_button, panel_action) in &mut interaction_q {
        match *interaction {
            Interaction::Pressed => {
                if let Some(slot_button) = slot_button {
                    current.file_name = Some(slot_button.file_name.clone());
                    color.0 = Color::srgb(0.35, 0.5, 0.85);
                }

                if let Some(action) = panel_action {
                    match action {
                        SavePanelAction::Save => {
                            if current.file_name.is_none() {
                                let new_index = slots.slots.len() as u32 + 1;
                                let display_name = generate_slot_display_name(new_index);
                                let file_name = format!("{display_name}.json");
                                current.file_name = Some(file_name);
                            }

                            ensure_slot_in_list(&mut slots, &current);
                            manual_save_writer.write(ManualSaveEvent);
                            color.0 = Color::srgb(0.25, 0.45, 0.8);
                        }
                        SavePanelAction::Close => {
                            if let Some(root) = panel_roots.iter().next() {
                                despawn_recursive(&mut commands, &children, root);
                            }
                            color.0 = Color::srgb(0.45, 0.45, 0.45);
                        }
                    }
                }
            }
            Interaction::Hovered => {
                if let Some(slot_button) = slot_button {
                    let is_selected = current
                        .file_name
                        .as_ref()
                        .map(|f| f == &slot_button.file_name)
                        .unwrap_or(false);
                    color.0 = if is_selected {
                        Color::srgb(0.35, 0.5, 0.85)
                    } else {
                        Color::srgb(0.4, 0.4, 0.5)
                    };
                }

                if let Some(action) = panel_action {
                    color.0 = match action {
                        SavePanelAction::Save => Color::srgb(0.45, 0.65, 0.95),
                        SavePanelAction::Close => Color::srgb(0.6, 0.6, 0.6),
                    };
                }
            }
            Interaction::None => {
                if let Some(slot_button) = slot_button {
                    let is_selected = current
                        .file_name
                        .as_ref()
                        .map(|f| f == &slot_button.file_name)
                        .unwrap_or(false);
                    color.0 = if is_selected {
                        Color::srgb(0.3, 0.45, 0.8)
                    } else {
                        Color::srgb(0.25, 0.25, 0.25)
                    };
                }

                if let Some(action) = panel_action {
                    color.0 = match action {
                        SavePanelAction::Save => Color::srgb(0.35, 0.55, 0.9),
                        SavePanelAction::Close => Color::srgb(0.5, 0.5, 0.5),
                    };
                }
            }
        }
    }
}

// -------------------------
// System B: only handle layout / node / text updates (Node / Children / Text)
// This avoids conflicting mutable borrows with BackgroundColor.
// -------------------------
fn handle_save_panel_layout(
    current: Res<CurrentSlot>,
    mut button_q: Query<
        (
            &Interaction,
            &mut BackgroundColor,
            Option<&SaveSlotButton>,
            Option<&SavePanelAction>,
        ),
        With<Button>,
    >,
) {
    for (interaction, mut color, slot_button, panel_action) in &mut button_q {
        if let Some(slot_button) = slot_button {
            let is_selected = current
                .file_name
                .as_ref()
                .map(|f| f == &slot_button.file_name)
                .unwrap_or(false);
            color.0 = match *interaction {
                Interaction::Pressed => Color::srgb(0.35, 0.5, 0.85),
                Interaction::Hovered => {
                    if is_selected {
                        Color::srgb(0.35, 0.5, 0.85)
                    } else {
                        Color::srgb(0.4, 0.4, 0.5)
                    }
                }
                Interaction::None => {
                    if is_selected {
                        Color::srgb(0.3, 0.45, 0.8)
                    } else {
                        Color::srgb(0.25, 0.25, 0.25)
                    }
                }
            };
        }

        if let Some(action) = panel_action {
            color.0 = match (*interaction, action) {
                (Interaction::Pressed, SavePanelAction::Save) => Color::srgb(0.25, 0.45, 0.8),
                (Interaction::Pressed, SavePanelAction::Close) => Color::srgb(0.45, 0.45, 0.45),
                (Interaction::Hovered, SavePanelAction::Save) => Color::srgb(0.45, 0.65, 0.95),
                (Interaction::Hovered, SavePanelAction::Close) => Color::srgb(0.6, 0.6, 0.6),
                (Interaction::None, SavePanelAction::Save) => Color::srgb(0.35, 0.55, 0.9),
                (Interaction::None, SavePanelAction::Close) => Color::srgb(0.5, 0.5, 0.5),
            };
        }
    }
}

/// 按 ESC 关闭设置面板（如果存在）
fn close_settings_on_esc(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut commands: Commands,
    panel: Query<Entity, With<SettingsPanel>>,
    children: Query<&Children>,
) {
    if !keyboard.just_pressed(KeyCode::Escape) {
        return;
    }

    if panel.is_empty() {
        return;
    }

    despawn_all::<SettingsPanel>(&mut commands, &panel, &children);
}

fn close_save_panel_on_esc(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut commands: Commands,
    panel: Query<Entity, With<SavePanel>>,
    children: Query<&Children>,
) {
    if !keyboard.just_pressed(KeyCode::Escape) {
        return;
    }

    if panel.is_empty() {
        return;
    }

    despawn_all::<SavePanel>(&mut commands, &panel, &children);
}

/// 退出主菜单 / 设置时清理
fn cleanup_main_menu(
    mut commands: Commands,
    menu: Query<Entity, With<MainMenuUI>>,
    panel: Query<Entity, With<SettingsPanel>>,
    save_panel: Query<Entity, With<SavePanel>>,
    children: Query<&Children>,
) {
    despawn_all::<MainMenuUI>(&mut commands, &menu, &children);
    despawn_all::<SettingsPanel>(&mut commands, &panel, &children);
    despawn_all::<SavePanel>(&mut commands, &save_panel, &children);
}

/// 退出暂停菜单 / 设置时清理
fn cleanup_pause_menu(
    mut commands: Commands,
    menu: Query<Entity, With<PauseMenuUI>>,
    panel: Query<Entity, With<SettingsPanel>>,
    save_panel: Query<Entity, With<SavePanel>>,
    children: Query<&Children>,
) {
    despawn_all::<PauseMenuUI>(&mut commands, &menu, &children);
    despawn_all::<SettingsPanel>(&mut commands, &panel, &children);
    despawn_all::<SavePanel>(&mut commands, &save_panel, &children);
}

/// 递归删除带某个标记组件的所有实体
fn despawn_all<T: Component>(
    commands: &mut Commands,
    targets: &Query<Entity, With<T>>,
    children: &Query<&Children>,
) {
    for entity in targets.iter() {
        despawn_recursive(commands, children, entity);
    }
}

/// 手写递归删除（避免依赖扩展 trait）
fn despawn_recursive(commands: &mut Commands, children: &Query<&Children>, entity: Entity) {
    if let Ok(child_entities) = children.get(entity) {
        for child in child_entities.iter() {
            despawn_recursive(commands, children, child);
        }
    }

    commands.entity(entity).despawn();
}
