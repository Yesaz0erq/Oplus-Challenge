use bevy::prelude::*;

use crate::health::Health;
use crate::i18n::{L10n, Language};
use crate::movement::Player;
use crate::state::GameState;
use crate::ui::EscBlockingUi;
use crate::ui::pause_menu::SuppressPauseMenuOnce;
use crate::ui::skin;
use crate::ui::types::GameSettings;

const DEBUG_HP_BOOST_MAX: f32 = 100_000.0;

pub struct DebugToolsPlugin;

#[derive(Resource, Default)]
pub struct DebugCheats {
    pub hp_boost_enabled: bool,
    pub noclip_enabled: bool,
    pub no_cooldown_enabled: bool,
    hp_restore_snapshot: Option<(f32, f32)>,
}

#[derive(Component)]
struct DebugMenuRoot;

#[derive(Component, Clone, Copy, PartialEq, Eq)]
enum DebugMenuAction {
    ToggleHpBoost,
    ToggleNoclip,
    ToggleNoCooldown,
    Close,
}

#[derive(Component, Clone, Copy, PartialEq, Eq)]
struct DebugMenuButton {
    action: DebugMenuAction,
}

#[derive(Component, Clone, Copy, PartialEq, Eq)]
struct DebugMenuText {
    action: DebugMenuAction,
}

impl Plugin for DebugToolsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<DebugCheats>()
            .add_systems(
                Update,
                (
                    toggle_debug_menu_hotkey,
                    close_debug_menu_on_esc.after(crate::input::EscInputSet),
                    handle_debug_menu_buttons,
                    sync_debug_menu_texts,
                    apply_hp_boost_on_player_spawn,
                ),
            )
            .add_systems(OnEnter(GameState::MainMenu), cleanup_debug_menu);
    }
}

fn toggle_debug_menu_hotkey(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut commands: Commands,
    root_q: Query<Entity, With<DebugMenuRoot>>,
    asset_server: Res<AssetServer>,
    settings: Res<GameSettings>,
    current_state: Res<State<GameState>>,
    mut next_state: ResMut<NextState<GameState>>,
    mut suppress_pause_menu_once: ResMut<SuppressPauseMenuOnce>,
) {
    let alt = keyboard.pressed(KeyCode::AltLeft) || keyboard.pressed(KeyCode::AltRight);
    if !(alt && keyboard.just_pressed(KeyCode::KeyC)) {
        return;
    }

    if let Ok(root) = root_q.single() {
        commands.entity(root).try_despawn();
        if matches!(current_state.get(), GameState::Paused) {
            suppress_pause_menu_once.0 = false;
            next_state.set(GameState::InGame);
        }
        return;
    }

    if !matches!(current_state.get(), GameState::InGame) {
        return;
    }

    spawn_debug_menu(&mut commands, &asset_server, settings.language);
    suppress_pause_menu_once.0 = true;
    next_state.set(GameState::Paused);
}

fn close_debug_menu_on_esc(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut commands: Commands,
    root_q: Query<Entity, With<DebugMenuRoot>>,
    current_state: Res<State<GameState>>,
    mut next_state: ResMut<NextState<GameState>>,
    mut suppress_pause_menu_once: ResMut<SuppressPauseMenuOnce>,
) {
    if !keyboard.just_pressed(KeyCode::Escape) {
        return;
    }

    let Ok(root) = root_q.single() else {
        return;
    };
    commands.entity(root).try_despawn();
    suppress_pause_menu_once.0 = false;
    if matches!(current_state.get(), GameState::InGame | GameState::Paused) {
        next_state.set(GameState::InGame);
    }
}

fn handle_debug_menu_buttons(
    mut interactions: Query<
        (&Interaction, &mut BackgroundColor, &DebugMenuButton),
        (Changed<Interaction>, With<Button>),
    >,
    mut cheats: ResMut<DebugCheats>,
    mut player_q: Query<&mut Health, With<Player>>,
    mut commands: Commands,
    root_q: Query<Entity, With<DebugMenuRoot>>,
    current_state: Res<State<GameState>>,
    mut next_state: ResMut<NextState<GameState>>,
    mut suppress_pause_menu_once: ResMut<SuppressPauseMenuOnce>,
) {
    for (interaction, mut bg, btn) in &mut interactions {
        match *interaction {
            Interaction::Pressed => {
                bg.0 = Color::srgb(0.85, 0.85, 0.95);
                match btn.action {
                    DebugMenuAction::ToggleHpBoost => toggle_hp_boost(&mut cheats, &mut player_q),
                    DebugMenuAction::ToggleNoclip => {
                        cheats.noclip_enabled = !cheats.noclip_enabled;
                    }
                    DebugMenuAction::ToggleNoCooldown => {
                        cheats.no_cooldown_enabled = !cheats.no_cooldown_enabled;
                    }
                    DebugMenuAction::Close => {
                        if let Ok(root) = root_q.single() {
                            commands.entity(root).try_despawn();
                            suppress_pause_menu_once.0 = false;
                            if matches!(current_state.get(), GameState::InGame | GameState::Paused)
                            {
                                next_state.set(GameState::InGame);
                            }
                        }
                    }
                }
            }
            Interaction::Hovered => bg.0 = Color::srgb(0.55, 0.55, 0.72),
            Interaction::None => bg.0 = base_button_color(btn.action),
        }
    }
}

fn sync_debug_menu_texts(
    cheats: Res<DebugCheats>,
    settings: Res<GameSettings>,
    mut text_q: Query<(&DebugMenuText, &mut Text)>,
) {
    let lang = settings.language;
    for (marker, mut text) in &mut text_q {
        *text = Text::new(debug_button_label(lang, marker.action, &cheats));
    }
}

fn apply_hp_boost_on_player_spawn(
    mut cheats: ResMut<DebugCheats>,
    mut q: Query<&mut Health, (With<Player>, Added<Player>)>,
) {
    if !cheats.hp_boost_enabled {
        return;
    }
    let Ok(mut hp) = q.single_mut() else {
        return;
    };
    if cheats.hp_restore_snapshot.is_none() {
        cheats.hp_restore_snapshot = Some((hp.current, hp.max));
    }
    hp.max = DEBUG_HP_BOOST_MAX;
    hp.current = DEBUG_HP_BOOST_MAX;
}

fn cleanup_debug_menu(mut commands: Commands, q: Query<Entity, With<DebugMenuRoot>>) {
    for e in q.iter() {
        commands.entity(e).try_despawn();
    }
}

fn toggle_hp_boost(cheats: &mut DebugCheats, player_q: &mut Query<&mut Health, With<Player>>) {
    cheats.hp_boost_enabled = !cheats.hp_boost_enabled;

    if cheats.hp_boost_enabled {
        if let Ok(mut hp) = player_q.single_mut() {
            cheats.hp_restore_snapshot = Some((hp.current, hp.max));
            hp.max = DEBUG_HP_BOOST_MAX;
            hp.current = DEBUG_HP_BOOST_MAX;
        }
        return;
    }

    let snapshot = cheats.hp_restore_snapshot.take();
    if let (Some((saved_current, saved_max)), Ok(mut hp)) = (snapshot, player_q.single_mut()) {
        hp.max = saved_max.max(1.0);
        hp.current = saved_current.clamp(0.0, hp.max);
    }
}

fn spawn_debug_menu(commands: &mut Commands, asset_server: &AssetServer, lang: Language) {
    let font = skin::ui_font(asset_server, lang);

    commands
        .spawn((
            DebugMenuRoot,
            EscBlockingUi,
            GlobalZIndex(130),
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                position_type: PositionType::Absolute,
                left: Val::Px(0.0),
                top: Val::Px(0.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(Color::srgba(0.02, 0.02, 0.03, 0.72)),
        ))
        .with_children(|root| {
            root.spawn((
                Node {
                    width: Val::Px(460.0),
                    min_height: Val::Px(300.0),
                    flex_direction: FlexDirection::Column,
                    row_gap: Val::Px(12.0),
                    padding: UiRect::all(Val::Px(16.0)),
                    border: UiRect::all(Val::Px(1.0)),
                    border_radius: BorderRadius::all(Val::Px(8.0)),
                    ..default()
                },
                BackgroundColor(Color::srgba(0.08, 0.09, 0.12, 0.96)),
                BorderColor::all(Color::srgb(0.45, 0.52, 0.68)),
            ))
            .with_children(|panel| {
                panel.spawn((
                    Text::new(L10n::debug_menu_title(lang)),
                    TextFont {
                        font: font.clone().into(),
                        font_size: FontSize::from(24.0),
                        ..default()
                    },
                    TextColor(Color::WHITE),
                ));

                panel.spawn((
                    Text::new(L10n::debug_menu_hotkey_hint(lang)),
                    TextFont {
                        font: font.clone().into(),
                        font_size: FontSize::from(13.0),
                        ..default()
                    },
                    TextColor(Color::srgba(1.0, 1.0, 1.0, 0.75)),
                ));

                spawn_debug_button(panel, &font, DebugMenuAction::ToggleHpBoost);
                spawn_debug_button(panel, &font, DebugMenuAction::ToggleNoclip);
                spawn_debug_button(panel, &font, DebugMenuAction::ToggleNoCooldown);
                spawn_debug_button(panel, &font, DebugMenuAction::Close);
            });
        });
}

fn spawn_debug_button(
    parent: &mut ChildSpawnerCommands<'_>,
    font: &Handle<Font>,
    action: DebugMenuAction,
) {
    parent
        .spawn((
            Button,
            DebugMenuButton { action },
            Node {
                width: Val::Percent(100.0),
                min_height: Val::Px(44.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                padding: UiRect::axes(Val::Px(10.0), Val::Px(8.0)),
                border_radius: BorderRadius::all(Val::Px(6.0)),
                ..default()
            },
            BackgroundColor(base_button_color(action)),
        ))
        .with_children(|btn| {
            btn.spawn((
                DebugMenuText { action },
                Text::new(""),
                TextFont {
                    font: font.clone().into(),
                    font_size: FontSize::from(18.0),
                    ..default()
                },
                TextColor(Color::WHITE),
            ));
        });
}

fn base_button_color(action: DebugMenuAction) -> Color {
    match action {
        DebugMenuAction::ToggleHpBoost => Color::srgb(0.42, 0.22, 0.22),
        DebugMenuAction::ToggleNoclip => Color::srgb(0.20, 0.34, 0.42),
        DebugMenuAction::ToggleNoCooldown => Color::srgb(0.25, 0.40, 0.22),
        DebugMenuAction::Close => Color::srgb(0.25, 0.25, 0.35),
    }
}

fn debug_button_label(lang: Language, action: DebugMenuAction, cheats: &DebugCheats) -> String {
    match action {
        DebugMenuAction::ToggleHpBoost => format!(
            "{}: {}",
            L10n::debug_hp_boost(lang),
            if cheats.hp_boost_enabled {
                L10n::settings_on(lang)
            } else {
                L10n::settings_off(lang)
            }
        ),
        DebugMenuAction::ToggleNoclip => format!(
            "{}: {}",
            L10n::debug_noclip(lang),
            if cheats.noclip_enabled {
                L10n::settings_on(lang)
            } else {
                L10n::settings_off(lang)
            }
        ),
        DebugMenuAction::ToggleNoCooldown => format!(
            "{}: {}",
            L10n::debug_no_cooldown(lang),
            if cheats.no_cooldown_enabled {
                L10n::settings_on(lang)
            } else {
                L10n::settings_off(lang)
            }
        ),
        DebugMenuAction::Close => L10n::close(lang).to_string(),
    }
}
