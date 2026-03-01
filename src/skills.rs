use bevy::prelude::*;
use bevy::input::mouse::MouseButton;
use bevy::window::PrimaryWindow;
use std::collections::{HashMap, HashSet};
use bevy::ecs::hierarchy::ChildOf;
use rand::Rng;

use crate::combat_core::{
    CombatSet, ProjectilePool, VfxPool, skill_light_wave, skill_slash, spawn_fireball_skill_projectile,
    spawn_light_wave_vfx, spawn_slash_vfx,
};
use crate::debug_tools::DebugCheats;
use crate::equipment::{EquipmentUiRoot, PlayerMemory};
use crate::enemy::{Enemy, EnemyAggro, EnemyHitbox};
use crate::health::Health;
use crate::i18n::L10n;
use crate::movement::{Player, PlayerAnimation, PlayerCamera, PlayerDash};
use crate::skills_pool::{SkillId, SkillPool};
use crate::state::GameState;
use crate::ui::pause_menu::SuppressPauseMenuOnce;
use crate::ui::skin;
use crate::ui::types::GameSettings;
use crate::ui::EscBlockingUi;

const MAX_SKILL_CARDS: usize = 3;
const SKILL_HUD_CARD_WIDTH: f32 = 78.0;
const SKILL_HUD_CARD_HEIGHT: f32 = 108.0;
const SKILL_HUD_CARD_GAP: f32 = 12.0;
const SKILL_BAG_PAGE_SIZE: usize = 10;
const SKILL_PARSE_COOLDOWN: f32 = 60.0;

#[derive(Component)]
struct SkillUiRoot;

#[derive(Component)]
struct SkillCard {
    slot_index: usize,
    skill: Option<SkillId>,
}

#[derive(Component)]
struct SkillNameText {
    slot_index: usize,
}

#[derive(Component)]
struct SkillCooldownText {
    slot_index: usize,
}

#[derive(Component)]
struct HpText;

#[derive(Component)]
struct DashCooldownHudText;

#[derive(Component)]
struct ParseCooldownHudText;

#[derive(Component)]
struct SkillBagUiRoot;

#[derive(Component)]
struct SkillBagPageButton {
    delta: i32,
}

#[derive(Component)]
struct SkillBagCloseButton;

#[derive(Component)]
struct SkillParseMarker;

#[derive(Component)]
struct SkillParsePopupRoot;

#[derive(Component, Clone, Copy)]
struct SkillParsePopupButton {
    action: SkillParsePopupAction,
}

#[derive(Clone, Copy)]
enum SkillParsePopupAction {
    Accept,
    Reject,
}

#[derive(Resource, Default)]
struct SkillBagPageState {
    current_page: usize,
}

#[derive(Resource, Default)]
struct SkillBagUiDirty(pub bool);

#[derive(Resource, Default)]
struct SkillBagOpenRequest(pub bool);

#[derive(Resource, Default)]
pub struct SkillParseState {
    pub cooldown: f32,
    pub selecting_target: bool,
}

#[derive(Resource, Default)]
struct SkillParseMarkerMap(HashMap<Entity, Entity>);

#[derive(Clone, Copy, Debug)]
pub struct SkillNumericStats {
    pub damage: f32,
    pub cooldown: f32,
}

impl SkillNumericStats {
    pub fn new(damage: f32, cooldown: f32) -> Self {
        Self { damage, cooldown }
    }
}

#[derive(Clone, Copy, Debug)]
struct SkillParseOffer {
    skill: SkillId,
    candidate: SkillNumericStats,
    current: Option<SkillNumericStats>,
}

#[derive(Resource, Default)]
struct SkillParsePopupState {
    offer: Option<SkillParseOffer>,
    wait_mouse_release: bool,
}

#[derive(Resource, Default)]
pub struct SkillRuntimeStats(pub HashMap<SkillId, SkillNumericStats>);

#[derive(Resource, Default)]
struct SkillParsePendingPick(Option<SkillId>);

#[derive(Component, Clone, Copy, Debug)]
pub struct ParseableSkill {
    pub skill: SkillId,
}

#[derive(Resource, Default)]
struct SkillCooldowns {
    slash_return: f32,
    fireball_return: f32,
    light_wave_return: f32,
}

impl SkillCooldowns {
    fn tick(&mut self, dt: f32) {
        self.slash_return = (self.slash_return - dt).max(0.0);
        self.fireball_return = (self.fireball_return - dt).max(0.0);
        self.light_wave_return = (self.light_wave_return - dt).max(0.0);
    }

    fn remaining(&self, skill: SkillId) -> f32 {
        match skill {
            SkillId::Dash => 0.0,
            SkillId::Slash => self.slash_return,
            SkillId::Fireball => self.fireball_return,
            SkillId::LightWave => self.light_wave_return,
        }
    }

    fn start(&mut self, skill: SkillId, seconds: f32) {
        match skill {
            SkillId::Dash => {}
            SkillId::Slash => {
                self.slash_return = seconds.max(self.slash_return);
            }
            SkillId::Fireball => {
                self.fireball_return = seconds.max(self.fireball_return);
            }
            SkillId::LightWave => {
                self.light_wave_return = seconds.max(self.light_wave_return);
            }
        }
    }
}

#[derive(Resource, Clone)]
pub struct CarriedSkills {
    pub slots: [Option<SkillId>; MAX_SKILL_CARDS],
}

impl Default for CarriedSkills {
    fn default() -> Self {
        Self {
            slots: [Some(SkillId::Slash), None, None],
        }
    }
}

pub struct SkillPlugin;

impl Plugin for SkillPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CarriedSkills>()
        .init_resource::<SkillCooldowns>()
        .init_resource::<SkillParseState>()
        .init_resource::<SkillParseMarkerMap>()
        .init_resource::<SkillParsePopupState>()
        .init_resource::<SkillParsePendingPick>()
        .init_resource::<SkillRuntimeStats>()
        .init_resource::<SkillBagPageState>()
        .init_resource::<SkillBagUiDirty>()
        .init_resource::<SkillBagOpenRequest>()
        .add_systems(OnEnter(GameState::InGame), setup_skill_ui)
        .add_systems(OnExit(GameState::InGame), cleanup_skill_ui)
        .add_systems(OnEnter(GameState::MainMenu), cleanup_skill_parse_popup_ui)
        .add_systems(OnEnter(GameState::GameOver), cleanup_skill_parse_popup_ui)
        .add_systems(OnExit(GameState::Paused), cleanup_skill_bag_ui)
        .add_systems(
            Update,
            (
                tick_skill_parse_cooldown,
                handle_skill_parse_input,
                process_skill_parse_pending_pick,
                sync_skill_parse_markers,
                spawn_other_skills,
                use_number_key_skills,
            )
                .chain()
                .in_set(CombatSet),
        )
        .add_systems(
            Update,
            (
                use_dash_skill_with_ctrl,
                update_hp_text,
                update_dash_cooldown_text,
                update_parse_cooldown_text,
                update_skill_cooldowns,
            )
                .in_set(CombatSet),
        )
        .add_systems(
            Update,
            (
                toggle_skill_bag_ui,
                close_skill_bag_ui_on_esc.after(crate::input::EscInputSet),
                close_skill_parse_popup_on_esc.after(crate::input::EscInputSet),
                handle_skill_parse_popup_buttons,
                process_skill_bag_open_request,
                handle_skill_bag_page_buttons,
                handle_skill_bag_close_button,
                rebuild_skill_bag_ui_when_dirty,
            )
                .run_if(skill_ui_in_game_or_paused),
        );
    }
}

fn skill_ui_in_game_or_paused(state: Res<State<GameState>>) -> bool {
    matches!(state.get(), GameState::InGame | GameState::Paused)
}

fn setup_skill_ui(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    settings: Res<GameSettings>,
    carried: Res<CarriedSkills>,
) {
    let lang = settings.language;
    let font: Handle<Font> = asset_server.load("fonts/YuFanLixing.otf");
    let hud_slot = skin::hud_slot(&asset_server);
    let mut initial_slots = [None; MAX_SKILL_CARDS];
    let mut write = 0usize;
    for skill in carried.slots.iter().copied().flatten() {
        if skill == SkillId::Dash || initial_slots.contains(&Some(skill)) {
            continue;
        }
        if write >= MAX_SKILL_CARDS {
            break;
        }
        initial_slots[write] = Some(skill);
        write += 1;
    }
    let root = commands
        .spawn((
            SkillUiRoot,
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                position_type: PositionType::Absolute,
                left: Val::Px(0.0),
                top: Val::Px(0.0),
                ..default()
            },
        ))
        .id();

    commands.entity(root).with_children(|parent| {
        parent.spawn((
            HpText,
            Text::new(L10n::skills_hp_label(lang)),
            TextFont {
                font: font.clone(),
                font_size: 18.0,
                ..default()
            },
            TextColor(skin::text_primary()),
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(16.0),
                top: Val::Px(16.0),
                ..default()
            },
        ));

        parent.spawn((
            DashCooldownHudText,
            Text::new(""),
            TextFont {
                font: font.clone(),
                font_size: 14.0,
                ..default()
            },
            TextColor(skin::text_muted()),
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(16.0),
                bottom: Val::Px(SKILL_HUD_CARD_HEIGHT + 54.0),
                ..default()
            },
        ));

        parent.spawn((
            ParseCooldownHudText,
            Text::new(""),
            TextFont {
                font: font.clone(),
                font_size: 14.0,
                ..default()
            },
            TextColor(skin::text_muted()),
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(16.0),
                bottom: Val::Px(SKILL_HUD_CARD_HEIGHT + 28.0),
                ..default()
            },
        ));

        parent
            .spawn((
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(16.0),
                    bottom: Val::Px(16.0),
                    flex_direction: FlexDirection::Row,
                    column_gap: Val::Px(SKILL_HUD_CARD_GAP),
                    align_items: AlignItems::FlexEnd,
                    ..default()
                },
            ))
            .with_children(|row| {
        for i in 0..MAX_SKILL_CARDS {
            row
                .spawn((
                    SkillCard {
                        slot_index: i,
                        skill: initial_slots[i],
                    },
                    Node {
                        width: Val::Px(SKILL_HUD_CARD_WIDTH),
                        height: Val::Px(SKILL_HUD_CARD_HEIGHT),
                        padding: UiRect::axes(Val::Px(8.0), Val::Px(10.0)),
                        flex_direction: FlexDirection::Column,
                        justify_content: JustifyContent::SpaceBetween,
                        align_items: AlignItems::Stretch,
                        ..default()
                    },
                    BackgroundColor(skin::inset_tint()),
                    ImageNode::new(hud_slot.clone()),
                ))
                .with_children(|card| {
                    card.spawn((
                        Node {
                            width: Val::Percent(100.0),
                            justify_content: JustifyContent::SpaceBetween,
                            align_items: AlignItems::Center,
                            ..default()
                        },
                    ))
                    .with_children(|header| {
                        header.spawn((
                            Text::new((i + 1).to_string()),
                            TextFont {
                                font: font.clone(),
                                font_size: 13.0,
                                ..default()
                            },
                            TextColor(skin::text_accent()),
                        ));
                        header.spawn((
                            Text::new("SKILL"),
                            TextFont {
                                font: font.clone(),
                                font_size: 8.0,
                                ..default()
                            },
                            TextColor(skin::text_muted()),
                        ));
                    });

                    card.spawn((
                        Node {
                            width: Val::Percent(100.0),
                            flex_grow: 1.0,
                            justify_content: JustifyContent::Center,
                            align_items: AlignItems::Center,
                            padding: UiRect::axes(Val::Px(4.0), Val::Px(6.0)),
                            ..default()
                        },
                    ))
                    .with_children(|body| {
                        body.spawn((
                            SkillNameText { slot_index: i },
                            Text::new(""),
                            TextLayout::new_with_justify(Justify::Center),
                            TextFont {
                                font: font.clone(),
                                font_size: 13.0,
                                ..default()
                            },
                            TextColor(skin::text_primary()),
                        ));
                    });

                    card.spawn((
                        SkillCooldownText { slot_index: i },
                        Text::new(""),
                        TextLayout::new_with_justify(Justify::Center),
                        TextFont {
                            font: font.clone(),
                            font_size: 10.0,
                            ..default()
                        },
                        TextColor(skin::text_muted()),
                        Node {
                            width: Val::Percent(100.0),
                            ..default()
                        },
                    ));
                });
        }
            });
    });
}

fn cleanup_skill_ui(mut commands: Commands, root_q: Query<Entity, With<SkillUiRoot>>) {
    for e in root_q.iter() {
        commands.entity(e).despawn();
    }
}

fn cleanup_skill_parse_popup_ui(
    mut commands: Commands,
    root_q: Query<Entity, With<SkillParsePopupRoot>>,
    mut popup_state: ResMut<SkillParsePopupState>,
) {
    for e in root_q.iter() {
        commands.entity(e).try_despawn();
    }
    popup_state.offer = None;
    popup_state.wait_mouse_release = false;
}

fn effective_skill_stats(
    skill: SkillId,
    pool: &SkillPool,
    runtime: &SkillRuntimeStats,
) -> SkillNumericStats {
    if let Some(stats) = runtime.0.get(&skill).copied() {
        return stats;
    }
    let def = pool.def(skill);
    SkillNumericStats {
        damage: def.damage,
        cooldown: def.cooldown,
    }
}

fn roll_skill_stats_around(base: SkillNumericStats) -> SkillNumericStats {
    let mut rng = rand::thread_rng();
    let damage_mul = rng.gen_range(0.75..=1.25);
    let cooldown_mul = rng.gen_range(0.75..=1.25);
    SkillNumericStats {
        damage: ((base.damage * damage_mul) * 10.0).round() / 10.0,
        cooldown: ((base.cooldown * cooldown_mul) * 10.0).round() / 10.0,
    }
}

fn spawn_skill_parse_popup_ui(
    commands: &mut Commands,
    asset_server: &AssetServer,
    lang: crate::i18n::Language,
    offer: SkillParseOffer,
    carried_count: usize,
    capacity: usize,
) {
    let font: Handle<Font> = asset_server.load("fonts/YuFanLixing.otf");
    let skill_name = L10n::skill_name(lang, offer.skill);
    let is_new_skill = offer.current.is_none();

    let mut desc = if is_new_skill {
        L10n::skill_parse_new_skill_desc(lang, skill_name)
    } else {
        L10n::skill_parse_owned_skill_desc(lang, skill_name)
    };

    if is_new_skill {
        desc.push_str(&format!(
            "\n{}: {}/{}",
            L10n::skill_backpack_capacity_label(lang),
            carried_count,
            capacity
        ));
        if carried_count >= capacity {
            desc.push_str(&format!("\n{}", L10n::skill_parse_bag_full(lang)));
        }
    }

    let action_label = if is_new_skill {
        L10n::skill_parse_save_to_bag(lang)
    } else {
        L10n::skill_parse_replace(lang)
    };

    let reject_label = if is_new_skill {
        L10n::skill_parse_discard(lang)
    } else {
        L10n::skill_parse_keep_old(lang)
    };

    commands
        .spawn((
            SkillParsePopupRoot,
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
            BackgroundColor(skin::overlay()),
        ))
        .with_children(|root| {
            root.spawn((
                Node {
                    width: Val::Px(760.0),
                    max_width: Val::Percent(90.0),
                    padding: UiRect::all(Val::Px(14.0)),
                    flex_direction: FlexDirection::Column,
                    row_gap: Val::Px(10.0),
                    ..default()
                },
                BackgroundColor(skin::panel_tint()),
                ImageNode::new(skin::panel(asset_server)),
            ))
            .with_children(|panel| {
                panel.spawn((
                    Text::new(L10n::skill_parse_popup_title(lang)),
                    TextFont {
                        font: font.clone(),
                        font_size: 24.0,
                        ..default()
                    },
                    TextColor(Color::WHITE),
                ));
                panel.spawn((
                    Text::new(desc),
                    TextFont {
                        font: font.clone(),
                        font_size: 14.0,
                        ..default()
                    },
                    TextColor(skin::text_muted()),
                ));
                panel
                    .spawn((
                        Node {
                            width: Val::Percent(100.0),
                            justify_content: JustifyContent::Center,
                            column_gap: Val::Px(12.0),
                            ..default()
                        },
                    ))
                    .with_children(|cards| {
                        if let Some(old) = offer.current {
                            spawn_parse_skill_card(
                                cards,
                                asset_server,
                                &font,
                                lang,
                                offer.skill,
                                L10n::skill_parse_old_values(lang),
                                old,
                                skin::slot_fill(),
                            );
                        }
                        spawn_parse_skill_card(
                            cards,
                            asset_server,
                            &font,
                            lang,
                            offer.skill,
                            L10n::skill_parse_new_values(lang),
                            offer.candidate,
                            skin::equipped_fill(),
                        );
                    });
                panel
                    .spawn((
                        Node {
                            width: Val::Percent(100.0),
                            justify_content: JustifyContent::FlexEnd,
                            column_gap: Val::Px(10.0),
                            ..default()
                        },
                    ))
                    .with_children(|buttons| {
                        buttons
                            .spawn((
                                Button,
                                SkillParsePopupButton {
                                    action: SkillParsePopupAction::Reject,
                                },
                                Node {
                                    width: Val::Px(120.0),
                                    height: Val::Px(34.0),
                                    justify_content: JustifyContent::Center,
                                    align_items: AlignItems::Center,
                                    ..default()
                                },
                                BackgroundColor(skin::button_idle()),
                                ImageNode::new(skin::button_large(asset_server)),
                            ))
                            .with_children(|btn| {
                                btn.spawn((
                                    Text::new(reject_label),
                                    TextFont {
                                        font: font.clone(),
                                        font_size: 14.0,
                                        ..default()
                                    },
                                    TextColor(skin::text_primary()),
                                ));
                            });

                        buttons
                            .spawn((
                                Button,
                                SkillParsePopupButton {
                                    action: SkillParsePopupAction::Accept,
                                },
                                Node {
                                    width: Val::Px(140.0),
                                    height: Val::Px(34.0),
                                    justify_content: JustifyContent::Center,
                                    align_items: AlignItems::Center,
                                    ..default()
                                },
                                BackgroundColor(skin::button_primary()),
                                ImageNode::new(skin::button_large(asset_server)),
                            ))
                            .with_children(|btn| {
                                btn.spawn((
                                    Text::new(action_label),
                                    TextFont {
                                        font: font.clone(),
                                        font_size: 14.0,
                                        ..default()
                                    },
                                    TextColor(skin::text_primary()),
                                ));
                            });
                    });
            });
        });
}

fn spawn_parse_skill_card(
    parent: &mut ChildSpawnerCommands<'_>,
    asset_server: &AssetServer,
    font: &Handle<Font>,
    lang: crate::i18n::Language,
    skill: SkillId,
    title: &str,
    stats: SkillNumericStats,
    border_color: Color,
) {
    let card_w = 210.0;
    let card_h = 290.0;

    parent
        .spawn((
            Node {
                width: Val::Px(card_w),
                height: Val::Px(card_h),
                padding: UiRect::all(Val::Px(2.0)),
                ..default()
            },
            BackgroundColor(border_color),
            ImageNode::new(skin::slot(asset_server)),
        ))
        .with_children(|card| {
            card.spawn((
                Node {
                    width: Val::Percent(100.0),
                    height: Val::Percent(100.0),
                    flex_direction: FlexDirection::Column,
                    justify_content: JustifyContent::FlexStart,
                    align_items: AlignItems::Stretch,
                    row_gap: Val::Px(6.0),
                    padding: UiRect::all(Val::Px(10.0)),
                    ..default()
                },
                BackgroundColor(skin::subpanel_tint()),
                ImageNode::new(skin::panel(asset_server)),
            ))
            .with_children(|content| {
                content.spawn((
                    Text::new(format!("{} [{}]", L10n::skill_name(lang, skill), title)),
                    TextFont {
                        font: font.clone(),
                        font_size: 16.0,
                        ..default()
                    },
                    TextColor(Color::WHITE),
                ));
                content.spawn((
                    Text::new(format!(
                        "{}: {}",
                        L10n::skill_card_effect(lang),
                        L10n::skill_effect_desc(lang, skill)
                    )),
                    TextFont {
                        font: font.clone(),
                        font_size: 11.0,
                        ..default()
                    },
                    TextColor(skin::text_muted()),
                ));
                content.spawn(card_stat_text(
                    font,
                    format!("{}: {:.1}", L10n::skill_card_damage(lang), stats.damage),
                ));
                content.spawn(card_stat_text(
                    font,
                    format!("{}: {:.1}s", L10n::skill_card_cooldown(lang), stats.cooldown),
                ));
            });
        });
}

fn tick_skill_parse_cooldown(
    time: Res<Time>,
    cheats: Option<Res<DebugCheats>>,
    mut parse_state: ResMut<SkillParseState>,
) {
    if cheats
        .as_ref()
        .map(|c| c.no_cooldown_enabled)
        .unwrap_or(false)
    {
        parse_state.cooldown = 0.0;
        return;
    }
    parse_state.cooldown = (parse_state.cooldown - time.delta_secs()).max(0.0);
}

fn handle_skill_parse_input(
    keyboard: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
    window_q: Query<&Window, With<PrimaryWindow>>,
    camera_q: Query<(&Camera, &GlobalTransform), With<PlayerCamera>>,
    mut parse_state: ResMut<SkillParseState>,
    parseable_q: Query<(&Transform, Option<&EnemyHitbox>, &ParseableSkill, Option<&Health>)>,
    popup_root_q: Query<Entity, With<SkillParsePopupRoot>>,
    popup_state: Res<SkillParsePopupState>,
    mut pending_pick: ResMut<SkillParsePendingPick>,
) {
    if !popup_root_q.is_empty() || popup_state.offer.is_some() {
        return;
    }

    if keyboard.just_pressed(KeyCode::KeyR) {
        if parse_state.selecting_target {
            parse_state.selecting_target = false;
            return;
        }
        if parse_state.cooldown <= 0.0 {
            parse_state.selecting_target = true;
        }
    }

    if !parse_state.selecting_target || !mouse.just_pressed(MouseButton::Left) {
        return;
    }

    let Some(cursor_world) = window_q
        .single()
        .ok()
        .and_then(|window| window.cursor_position())
        .and_then(|cursor| {
            camera_q
                .single()
                .ok()
                .and_then(|(cam, cam_tf)| cam.viewport_to_world_2d(cam_tf, cursor).ok())
        })
    else {
        return;
    };

    let mut picked = None;
    let mut best_dist = f32::MAX;

    for (tf, hitbox, parseable, hp) in &parseable_q {
        if hp.map(|v| v.current <= 0.0).unwrap_or(false) {
            continue;
        }
        let target_pos = tf.translation.truncate();
        let dist = target_pos.distance(cursor_world);
        let pick_radius = hitbox.map(|h| h.half.max_element() + 12.0).unwrap_or(20.0);
        if dist <= pick_radius && dist < best_dist {
            best_dist = dist;
            picked = Some(parseable.skill);
        }
    }

    let Some(skill) = picked else {
        return;
    };

    parse_state.cooldown = SKILL_PARSE_COOLDOWN;
    parse_state.selecting_target = false;
    pending_pick.0 = Some(skill);
}

fn process_skill_parse_pending_pick(
    mut pending_pick: ResMut<SkillParsePendingPick>,
    mut popup_state: ResMut<SkillParsePopupState>,
    carried: Res<CarriedSkills>,
    player_memory_q: Query<&PlayerMemory, With<Player>>,
    pool: Res<SkillPool>,
    skill_runtime: Res<SkillRuntimeStats>,
    asset_server: Res<AssetServer>,
    settings: Res<GameSettings>,
    popup_root_q: Query<Entity, With<SkillParsePopupRoot>>,
    current_state: Res<State<GameState>>,
    mut next_state: ResMut<NextState<GameState>>,
    mut commands: Commands,
) {
    let Some(skill) = pending_pick.0.take() else {
        return;
    };
    if !popup_root_q.is_empty() || popup_state.offer.is_some() {
        return;
    }

    let cap = player_memory_q
        .single()
        .ok()
        .map(|memory| memory.skill_capacity)
        .unwrap_or(MAX_SKILL_CARDS);
    let carried_count = carried_non_dash_count(&carried);

    let had_skill = carried.slots.contains(&Some(skill));
    let current_stats = if had_skill {
        Some(effective_skill_stats(skill, &pool, &skill_runtime))
    } else {
        None
    };
    let candidate = if let Some(current) = current_stats {
        roll_skill_stats_around(current)
    } else {
        effective_skill_stats(skill, &pool, &skill_runtime)
    };
    let offer = SkillParseOffer {
        skill,
        candidate,
        current: current_stats,
    };
    popup_state.offer = Some(offer);
    popup_state.wait_mouse_release = true;

    spawn_skill_parse_popup_ui(
        &mut commands,
        &asset_server,
        settings.language,
        offer,
        carried_count,
        cap,
    );

    if matches!(current_state.get(), GameState::InGame) {
        next_state.set(GameState::Paused);
    }
}

fn sync_skill_parse_markers(
    mut commands: Commands,
    parseable_q: Query<(Entity, Option<&Sprite>, Option<&Health>), With<ParseableSkill>>,
    mut marker_map: ResMut<SkillParseMarkerMap>,
    marker_q: Query<(), With<SkillParseMarker>>,
) {
    let mut alive_targets = HashSet::new();

    for (target, sprite, hp) in &parseable_q {
        if hp.map(|v| v.current <= 0.0).unwrap_or(false) {
            continue;
        }
        alive_targets.insert(target);

        if marker_map.0.contains_key(&target) {
            continue;
        }

        let marker_y = sprite
            .and_then(|s| s.custom_size)
            .map(|size| size.y * 0.5 + 16.0)
            .unwrap_or(26.0);

        let mut marker_sprite = Sprite::default();
        marker_sprite.color = skin::text_accent();
        marker_sprite.custom_size = Some(Vec2::new(10.0, 10.0));

        let marker = commands
            .spawn((
                SkillParseMarker,
                ChildOf(target),
                marker_sprite,
                Transform::from_xyz(0.0, marker_y, 140.0),
            ))
            .id();

        marker_map.0.insert(target, marker);
    }

    let stale: Vec<(Entity, Entity)> = marker_map
        .0
        .iter()
        .filter_map(|(target, marker)| {
            if !alive_targets.contains(target) || marker_q.get(*marker).is_err() {
                Some((*target, *marker))
            } else {
                None
            }
        })
        .collect();

    for (target, marker) in stale {
        marker_map.0.remove(&target);
        if marker_q.get(marker).is_ok() {
            commands.entity(marker).try_despawn();
        }
    }
}

fn add_parsed_skill(carried: &mut CarriedSkills, skill: SkillId, capacity: usize) -> bool {
    if skill == SkillId::Dash || carried.slots.contains(&Some(skill)) {
        return false;
    }

    let cap = capacity.min(MAX_SKILL_CARDS);
    let used = carried_non_dash_count(carried);
    if used >= cap {
        return false;
    }

    if let Some(idx) = carried.slots.iter().position(Option::is_none) {
        carried.slots[idx] = Some(skill);
        return true;
    }

    false
}

fn spawn_other_skills(
    time: Res<Time>,
    mut cooldowns: ResMut<SkillCooldowns>,
    cheats: Option<Res<DebugCheats>>,
    mut cards_q: Query<&mut SkillCard>,
    carried: Res<CarriedSkills>,
) {
    if cheats
        .as_ref()
        .map(|c| c.no_cooldown_enabled)
        .unwrap_or(false)
    {
        cooldowns.slash_return = 0.0;
        cooldowns.fireball_return = 0.0;
        cooldowns.light_wave_return = 0.0;
    } else {
        cooldowns.tick(time.delta_secs());
    }

    let mut slots = [None; MAX_SKILL_CARDS];
    for c in &mut cards_q {
        if c.slot_index < MAX_SKILL_CARDS {
            slots[c.slot_index] = c.skill;
        }
    }

    let mut seen_owned = [None; MAX_SKILL_CARDS];
    let mut seen_count = 0usize;
    for skill in carried.slots.iter().copied().flatten() {
        if skill == SkillId::Dash {
            continue;
        }
        if seen_owned.contains(&Some(skill)) {
            continue;
        }
        if cooldowns.remaining(skill) > 0.0 {
            continue;
        }
        if slots.contains(&Some(skill)) {
            continue;
        }
        if let Some(empty_idx) = slots.iter().position(Option::is_none) {
            slots[empty_idx] = Some(skill);
            if seen_count < MAX_SKILL_CARDS {
                seen_owned[seen_count] = Some(skill);
                seen_count += 1;
            }
        }
    }

    for mut c in &mut cards_q {
        if c.slot_index < MAX_SKILL_CARDS {
            c.skill = slots[c.slot_index];
        }
    }
}

fn use_number_key_skills(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut cooldowns: ResMut<SkillCooldowns>,
    cheats: Option<Res<DebugCheats>>,
    mut cards_q: Query<&mut SkillCard>,
    mut player_q: Query<(&Transform, &mut PlayerAnimation), With<Player>>,
    window_q: Query<&Window, With<PrimaryWindow>>,
    camera_q: Query<(&Camera, &GlobalTransform), With<PlayerCamera>>,
    mut enemies_q: Query<(Entity, &Transform, &mut Health, &mut EnemyAggro), With<Enemy>>,
    mut commands: Commands,
    pool: Res<SkillPool>,
    runtime_stats: Res<SkillRuntimeStats>,
    mut projectile_pool: ResMut<ProjectilePool>,
    mut vfx_pool: ResMut<VfxPool>,
) {
    let Ok((player_tf, anim)) = player_q.single_mut() else {
        return;
    };
    let origin = player_tf.translation.truncate();
    let facing_dir = anim.direction.as_vec2().normalize_or_zero();
    let cursor_dir = window_q
        .single()
        .ok()
        .and_then(|window| window.cursor_position())
        .and_then(|cursor| {
            camera_q
                .single()
                .ok()
                .and_then(|(cam, cam_tf)| cam.viewport_to_world_2d(cam_tf, cursor).ok())
        })
        .map(|world_pos| (world_pos - origin).normalize_or_zero())
        .filter(|dir| *dir != Vec2::ZERO);

    let keys = [KeyCode::Digit1, KeyCode::Digit2, KeyCode::Digit3];
    let no_cooldown = cheats
        .as_ref()
        .map(|c| c.no_cooldown_enabled)
        .unwrap_or(false);

    for (slot, key) in keys.iter().enumerate() {
        if !keyboard.just_pressed(*key) {
            continue;
        }

        let mut skill = None;

        for c in &mut cards_q {
            if c.slot_index == slot {
                skill = c.skill;
                break;
            }
        }

        let Some(skill) = skill else {
            continue;
        };
        let stats = effective_skill_stats(skill, &pool, &runtime_stats);

        let used_cooldown = match skill {
            SkillId::Slash => {
                let dir = cursor_dir.unwrap_or(facing_dir);
                spawn_slash_vfx(&mut commands, Some(&mut vfx_pool), origin, dir);
                skill_slash(origin, dir, &mut enemies_q, stats.damage);
                stats.cooldown
            }
            SkillId::Fireball => {
                let dir = cursor_dir.unwrap_or(facing_dir);
                spawn_fireball_skill_projectile(
                    &mut commands,
                    Some(&mut projectile_pool),
                    origin,
                    dir,
                    stats.damage,
                );
                stats.cooldown
            }
            SkillId::LightWave => {
                let dir = cursor_dir.unwrap_or(facing_dir);
                spawn_light_wave_vfx(&mut commands, Some(&mut vfx_pool), origin, dir);
                skill_light_wave(origin, dir, &mut enemies_q, stats.damage);
                stats.cooldown
            }
            SkillId::Dash => 0.0,
        };

        let mut slots = [None; MAX_SKILL_CARDS];
        for c in &mut cards_q {
            if c.slot_index < MAX_SKILL_CARDS {
                slots[c.slot_index] = c.skill;
            }
        }
        for i in slot..(MAX_SKILL_CARDS - 1) {
            slots[i] = slots[i + 1];
        }
        slots[MAX_SKILL_CARDS - 1] = None;
        if !no_cooldown {
            cooldowns.start(skill, used_cooldown);
        }

        for mut c in &mut cards_q {
            if c.slot_index < MAX_SKILL_CARDS {
                c.skill = slots[c.slot_index];
            }
        }
    }
}

fn use_dash_skill_with_ctrl(
    keyboard: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    cheats: Option<Res<DebugCheats>>,
    mut player_q: Query<(Entity, &mut PlayerDash, &mut PlayerAnimation), With<Player>>,
) {
    let Ok((_e, mut dash, anim)) = player_q.single_mut() else {
        return;
    };

    let no_cooldown = cheats
        .as_ref()
        .map(|c| c.no_cooldown_enabled)
        .unwrap_or(false);

    if no_cooldown {
        dash.cooldown = 0.0;
    } else {
        dash.cooldown = (dash.cooldown - time.delta_secs()).max(0.0);
    }

    if keyboard.just_pressed(KeyCode::ControlLeft) && dash.cooldown <= 0.0 {
        let dir_vec = anim.direction.as_vec2();
        let dir = if dir_vec == Vec2::ZERO {
            Vec2::Y
        } else {
            dir_vec
        };

        dash.is_dashing = true;
        dash.remaining = crate::movement::DASH_DURATION;
        dash.direction = dir;
        dash.cooldown = if no_cooldown {
            0.0
        } else {
            crate::movement::DASH_COOLDOWN
        };
    }
}

fn update_hp_text(
    mut q: Query<&mut Text, With<HpText>>,
    player_q: Query<&Health, With<Player>>,
    settings: Res<GameSettings>,
) {
    let Ok(player_hp) = player_q.single() else {
        return;
    };
    for mut t in &mut q {
        *t = Text::new(L10n::hp_short(
            settings.language,
            player_hp.current,
            player_hp.max,
        ));
    }
}

fn update_dash_cooldown_text(
    mut q: Query<&mut Text, With<DashCooldownHudText>>,
    player_q: Query<&PlayerDash, With<Player>>,
    settings: Res<GameSettings>,
) {
    let Ok(dash) = player_q.single() else {
        return;
    };
    let label = L10n::skill_name(settings.language, SkillId::Dash);
    let cooldown_label = L10n::skill_card_cooldown(settings.language);
    for mut t in &mut q {
        let cd = dash.cooldown.max(0.0);
        let value = if cd <= 0.0 {
            L10n::skill_parse_ready(settings.language).to_string()
        } else {
            format!("{cd:.1}s")
        };
        *t = Text::new(format!("{label}{cooldown_label}: {value}"));
    }
}

fn update_parse_cooldown_text(
    mut q: Query<&mut Text, With<ParseCooldownHudText>>,
    parse_state: Res<SkillParseState>,
    settings: Res<GameSettings>,
) {
    let lang = settings.language;
    for mut text in &mut q {
        let value = if parse_state.selecting_target {
            L10n::skill_parse_selecting(lang).to_string()
        } else if parse_state.cooldown > 0.0 {
            format!("{:.1}s", parse_state.cooldown)
        } else {
            L10n::skill_parse_ready(lang).to_string()
        };
        *text = Text::new(format!(
            "{}{}: {}",
            L10n::skill_parse_name(lang),
            L10n::skill_card_cooldown(lang),
            value
        ));
    }
}

fn update_skill_cooldowns(
    cards_q: Query<&SkillCard>,
    cooldowns: Res<SkillCooldowns>,
    mut text_q: ParamSet<(
        Query<(&SkillNameText, &mut Text)>,
        Query<(&SkillCooldownText, &mut Text)>,
    )>,
    settings: Res<GameSettings>,
) {
    let lang = settings.language;

    for (marker, mut name_text) in &mut text_q.p0() {
        let skill = cards_q
            .iter()
            .find(|c| c.slot_index == marker.slot_index)
            .and_then(|c| c.skill);

        *name_text = Text::new(match skill {
            Some(skill) => L10n::skill_name(lang, skill).to_string(),
            None => {
                if lang == crate::i18n::Language::ZhCn {
                    "空卡槽".to_string()
                } else {
                    "Empty Slot".to_string()
                }
            }
        });
    }

    for (marker, mut cooldown_text) in &mut text_q.p1() {
        let skill = cards_q
            .iter()
            .find(|c| c.slot_index == marker.slot_index)
            .and_then(|c| c.skill);

        let label = match skill {
            Some(skill) => {
                let remaining = cooldowns.remaining(skill);
                if remaining > 0.0 {
                    format!("{} {:.1}s", L10n::skill_card_cooldown(lang), remaining)
                } else if lang == crate::i18n::Language::ZhCn {
                    "就绪".to_string()
                } else {
                    "Ready".to_string()
                }
            }
            None => {
                if lang == crate::i18n::Language::ZhCn {
                    "未装备".to_string()
                } else {
                    "No Skill".to_string()
                }
            }
        };

        *cooldown_text = Text::new(label);
    }
}

fn toggle_skill_bag_ui(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut commands: Commands,
    skill_bag_q: Query<Entity, With<SkillBagUiRoot>>,
    equipment_ui_q: Query<Entity, With<EquipmentUiRoot>>,
    asset_server: Res<AssetServer>,
    pool: Res<SkillPool>,
    runtime_stats: Res<SkillRuntimeStats>,
    carried: Res<CarriedSkills>,
    player_q: Query<&PlayerMemory, With<Player>>,
    mut page_state: ResMut<SkillBagPageState>,
    mut dirty: ResMut<SkillBagUiDirty>,
    settings: Res<GameSettings>,
    current_state: Res<State<GameState>>,
    mut next_state: ResMut<NextState<GameState>>,
    mut suppress_pause_menu_once: ResMut<SuppressPauseMenuOnce>,
) {
    if !keyboard.just_pressed(KeyCode::KeyV) {
        return;
    }

    if let Ok(root) = skill_bag_q.single() {
        commands.entity(root).try_despawn();
        dirty.0 = false;
        if matches!(current_state.get(), GameState::Paused) {
            suppress_pause_menu_once.0 = false;
            next_state.set(GameState::InGame);
        }
        return;
    }

    if !equipment_ui_q.is_empty() {
        return;
    }

    if matches!(current_state.get(), GameState::Paused) {
        return;
    }

    let Ok(memory) = player_q.single() else {
        return;
    };

    page_state.current_page = 0;
    dirty.0 = false;
    spawn_skill_bag_ui(
        &mut commands,
        &asset_server,
        &pool,
        &runtime_stats,
        &carried,
        memory,
        &mut page_state,
        settings.language,
    );
    suppress_pause_menu_once.0 = true;
    next_state.set(GameState::Paused);
}

fn close_skill_bag_ui_on_esc(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut commands: Commands,
    q: Query<Entity, With<SkillBagUiRoot>>,
    current_state: Res<State<GameState>>,
    mut next_state: ResMut<NextState<GameState>>,
    mut suppress_pause_menu_once: ResMut<SuppressPauseMenuOnce>,
) {
    if !keyboard.just_pressed(KeyCode::Escape) {
        return;
    }

    if let Ok(root) = q.single() {
        commands.entity(root).try_despawn();
        suppress_pause_menu_once.0 = false;
        if matches!(current_state.get(), GameState::InGame | GameState::Paused) {
            next_state.set(GameState::InGame);
        }
    }
}

fn close_skill_parse_popup_on_esc(
    keyboard: Res<ButtonInput<KeyCode>>,
    popup_q: Query<Entity, With<SkillParsePopupRoot>>,
    mut popup_state: ResMut<SkillParsePopupState>,
    mut commands: Commands,
) {
    if !keyboard.just_pressed(KeyCode::Escape) {
        return;
    }

    let Ok(root) = popup_q.single() else {
        return;
    };

    commands.entity(root).try_despawn();
    popup_state.offer = None;
    popup_state.wait_mouse_release = false;
}

fn handle_skill_parse_popup_buttons(
    mouse: Res<ButtonInput<MouseButton>>,
    mut interactions: Query<
        (&Interaction, &mut BackgroundColor, &SkillParsePopupButton),
        (Changed<Interaction>, With<Button>),
    >,
    popup_q: Query<Entity, With<SkillParsePopupRoot>>,
    mut popup_state: ResMut<SkillParsePopupState>,
    mut carried: ResMut<CarriedSkills>,
    mut skill_runtime: ResMut<SkillRuntimeStats>,
    player_memory_q: Query<&PlayerMemory, With<Player>>,
    mut request_open_bag: ResMut<SkillBagOpenRequest>,
    mut suppress_pause_menu_once: ResMut<SuppressPauseMenuOnce>,
    mut dirty: ResMut<SkillBagUiDirty>,
    mut commands: Commands,
    mut next_state: ResMut<NextState<GameState>>,
) {
    let Ok(root) = popup_q.single() else {
        return;
    };
    let Some(offer) = popup_state.offer else {
        return;
    };
    if popup_state.wait_mouse_release {
        if mouse.pressed(MouseButton::Left) {
            return;
        }
        popup_state.wait_mouse_release = false;
    }

    for (interaction, mut bg, button) in &mut interactions {
        match *interaction {
            Interaction::Pressed => {
                bg.0 = skin::button_pressed();
                match button.action {
                    SkillParsePopupAction::Reject => {
                        popup_state.offer = None;
                        popup_state.wait_mouse_release = false;
                        commands.entity(root).try_despawn();
                        suppress_pause_menu_once.0 = false;
                        next_state.set(GameState::InGame);
                        return;
                    }
                    SkillParsePopupAction::Accept => {
                        let mut changed = false;
                        if offer.current.is_none() {
                            let cap = player_memory_q
                                .single()
                                .ok()
                                .map(|memory| memory.skill_capacity)
                                .unwrap_or(MAX_SKILL_CARDS);
                            if add_parsed_skill(&mut carried, offer.skill, cap) {
                                skill_runtime.0.insert(offer.skill, offer.candidate);
                                changed = true;
                            }
                        } else {
                            skill_runtime.0.insert(offer.skill, offer.candidate);
                            changed = true;
                        }

                        // Save failed (e.g. bag full): keep popup open so player can choose discard.
                        if !changed {
                            continue;
                        }

                        dirty.0 = true;
                        popup_state.offer = None;
                        popup_state.wait_mouse_release = false;
                        commands.entity(root).try_despawn();
                        request_open_bag.0 = true;
                        suppress_pause_menu_once.0 = false;
                        next_state.set(GameState::InGame);
                        return;
                    }
                }
            }
            Interaction::Hovered => bg.0 = skin::button_hover(),
            Interaction::None => {
                let base = match button.action {
                    SkillParsePopupAction::Accept => skin::button_primary(),
                    SkillParsePopupAction::Reject => skin::button_idle(),
                };
                bg.0 = base;
            }
        }
    }
}

fn process_skill_bag_open_request(
    mut commands: Commands,
    mut request_open_bag: ResMut<SkillBagOpenRequest>,
    skill_bag_q: Query<Entity, With<SkillBagUiRoot>>,
    equipment_ui_q: Query<Entity, With<EquipmentUiRoot>>,
    popup_q: Query<Entity, With<SkillParsePopupRoot>>,
    asset_server: Res<AssetServer>,
    pool: Res<SkillPool>,
    runtime_stats: Res<SkillRuntimeStats>,
    carried: Res<CarriedSkills>,
    player_q: Query<&PlayerMemory, With<Player>>,
    mut page_state: ResMut<SkillBagPageState>,
    mut dirty: ResMut<SkillBagUiDirty>,
    settings: Res<GameSettings>,
    current_state: Res<State<GameState>>,
    mut next_state: ResMut<NextState<GameState>>,
    mut suppress_pause_menu_once: ResMut<SuppressPauseMenuOnce>,
) {
    if !request_open_bag.0 {
        return;
    }

    for popup in &popup_q {
        commands.entity(popup).try_despawn();
    }

    if !equipment_ui_q.is_empty() {
        for root in &equipment_ui_q {
            commands.entity(root).try_despawn();
        }
    }

    if !skill_bag_q.is_empty() {
        request_open_bag.0 = false;
        return;
    }

    if matches!(current_state.get(), GameState::Paused) {
        suppress_pause_menu_once.0 = false;
        next_state.set(GameState::InGame);
        return;
    }

    let Ok(memory) = player_q.single() else {
        request_open_bag.0 = false;
        return;
    };

    page_state.current_page = 0;
    dirty.0 = false;
    spawn_skill_bag_ui(
        &mut commands,
        &asset_server,
        &pool,
        &runtime_stats,
        &carried,
        memory,
        &mut page_state,
        settings.language,
    );
    suppress_pause_menu_once.0 = true;
    next_state.set(GameState::Paused);
    request_open_bag.0 = false;
}

fn handle_skill_bag_page_buttons(
    mut interactions: Query<
        (&Interaction, &mut BackgroundColor, &SkillBagPageButton),
        (Changed<Interaction>, With<Button>),
    >,
    skill_bag_q: Query<Entity, With<SkillBagUiRoot>>,
    carried: Res<CarriedSkills>,
    mut page_state: ResMut<SkillBagPageState>,
    mut dirty: ResMut<SkillBagUiDirty>,
) {
    if skill_bag_q.is_empty() {
        return;
    }

    let total_cards = carried_skill_types_for_bag(&carried).len().max(1);
    let total_pages = total_cards.div_ceil(SKILL_BAG_PAGE_SIZE);

    for (interaction, mut bg, btn) in &mut interactions {
        match *interaction {
            Interaction::Pressed => {
                bg.0 = skin::button_pressed();
                let old = page_state.current_page;
                let max_page = total_pages.saturating_sub(1);
                let next = if btn.delta < 0 {
                    old.saturating_sub(btn.delta.unsigned_abs() as usize)
                } else {
                    old.saturating_add(btn.delta as usize).min(max_page)
                };
                if next != old {
                    page_state.current_page = next;
                    dirty.0 = true;
                }
            }
            Interaction::Hovered => bg.0 = skin::button_hover(),
            Interaction::None => bg.0 = skin::button_idle(),
        }
    }
}

fn handle_skill_bag_close_button(
    mut interactions: Query<
        (&Interaction, &mut BackgroundColor),
        (Changed<Interaction>, With<Button>, With<SkillBagCloseButton>),
    >,
    mut commands: Commands,
    root_q: Query<Entity, With<SkillBagUiRoot>>,
    current_state: Res<State<GameState>>,
    mut next_state: ResMut<NextState<GameState>>,
    mut suppress_pause_menu_once: ResMut<SuppressPauseMenuOnce>,
    mut dirty: ResMut<SkillBagUiDirty>,
) {
    for (interaction, mut bg) in &mut interactions {
        match *interaction {
            Interaction::Pressed => {
                bg.0 = skin::button_pressed();
                if let Ok(root) = root_q.single() {
                    commands.entity(root).try_despawn();
                    dirty.0 = false;
                    suppress_pause_menu_once.0 = false;
                    if matches!(current_state.get(), GameState::InGame | GameState::Paused) {
                        next_state.set(GameState::InGame);
                    }
                }
            }
            Interaction::Hovered => bg.0 = skin::button_hover(),
            Interaction::None => bg.0 = skin::button_idle(),
        }
    }
}

fn rebuild_skill_bag_ui_when_dirty(
    mut commands: Commands,
    root_q: Query<Entity, With<SkillBagUiRoot>>,
    asset_server: Res<AssetServer>,
    pool: Res<SkillPool>,
    runtime_stats: Res<SkillRuntimeStats>,
    carried: Res<CarriedSkills>,
    player_q: Query<&PlayerMemory, With<Player>>,
    mut page_state: ResMut<SkillBagPageState>,
    mut dirty: ResMut<SkillBagUiDirty>,
    settings: Res<GameSettings>,
) {
    if root_q.is_empty() {
        dirty.0 = false;
        return;
    }

    if settings.is_changed() {
        dirty.0 = true;
    }

    if !dirty.0 {
        return;
    }

    if let Ok(root) = root_q.single() {
        commands.entity(root).try_despawn();
    }
    let Ok(memory) = player_q.single() else {
        return;
    };
    spawn_skill_bag_ui(
        &mut commands,
        &asset_server,
        &pool,
        &runtime_stats,
        &carried,
        memory,
        &mut page_state,
        settings.language,
    );
    dirty.0 = false;
}

fn cleanup_skill_bag_ui(mut commands: Commands, q: Query<Entity, With<SkillBagUiRoot>>) {
    for e in q.iter() {
        commands.entity(e).try_despawn();
    }
}

fn carried_skill_types_for_bag(carried: &CarriedSkills) -> Vec<SkillId> {
    let mut out = Vec::new();
    for id in carried.slots.iter().copied().flatten() {
        if id == SkillId::Dash || out.contains(&id) {
            continue;
        }
        out.push(id);
    }
    out
}

fn carried_non_dash_count(carried: &CarriedSkills) -> usize {
    carried
        .slots
        .iter()
        .copied()
        .flatten()
        .filter(|id| *id != SkillId::Dash)
        .count()
}

fn spawn_skill_bag_ui(
    commands: &mut Commands,
    asset_server: &AssetServer,
    pool: &SkillPool,
    runtime_stats: &SkillRuntimeStats,
    carried: &CarriedSkills,
    memory: &PlayerMemory,
    page_state: &mut SkillBagPageState,
    lang: crate::i18n::Language,
) {
    let font: Handle<Font> = asset_server.load("fonts/YuFanLixing.otf");
    let carried_list = carried_skill_types_for_bag(carried);
    let carried_count = carried_non_dash_count(carried);
    let total_cards = carried_list.len().max(1);
    let total_pages = total_cards.div_ceil(SKILL_BAG_PAGE_SIZE);
    page_state.current_page = page_state.current_page.min(total_pages.saturating_sub(1));
    let current_page = page_state.current_page;
    let start = current_page * SKILL_BAG_PAGE_SIZE;

    let card_w = 150.0;
    let card_h = 220.0;

    commands
        .spawn((
            SkillBagUiRoot,
            EscBlockingUi,
            GlobalZIndex(120),
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
            BackgroundColor(skin::overlay()),
        ))
        .with_children(|root| {
            root.spawn((
                Node {
                    width: Val::Percent(78.0),
                    max_width: Val::Px(980.0),
                    height: Val::Percent(72.0),
                    max_height: Val::Px(620.0),
                    flex_direction: FlexDirection::Column,
                    row_gap: Val::Px(10.0),
                    padding: UiRect::all(Val::Px(12.0)),
                    ..default()
                },
                BackgroundColor(skin::panel_tint()),
                ImageNode::new(skin::panel(asset_server)),
            ))
            .with_children(|panel| {
                panel.spawn((
                    Node {
                        width: Val::Percent(100.0),
                        justify_content: JustifyContent::SpaceBetween,
                        align_items: AlignItems::FlexStart,
                        ..default()
                    },
                ))
                .with_children(|header| {
                    header
                        .spawn((
                            Node {
                                flex_direction: FlexDirection::Column,
                                row_gap: Val::Px(6.0),
                                align_items: AlignItems::FlexStart,
                                ..default()
                            },
                        ))
                        .with_children(|left| {
                            left.spawn((
                                Text::new(L10n::skill_backpack_title(lang)),
                                TextFont {
                                    font: font.clone(),
                                    font_size: 24.0,
                                    ..default()
                                },
                                TextColor(skin::text_primary()),
                            ));

                            left.spawn((
                                Text::new(L10n::skill_backpack_capacity(
                                    lang,
                                    carried_count,
                                    memory.skill_capacity,
                                )),
                                TextFont {
                                    font: font.clone(),
                                    font_size: 16.0,
                                    ..default()
                                },
                                TextColor(skin::text_muted()),
                            ));
                        });

                    header
                        .spawn((
                            Node {
                                width: Val::Px(280.0),
                                flex_direction: FlexDirection::Column,
                                row_gap: Val::Px(6.0),
                                align_items: AlignItems::FlexEnd,
                                ..default()
                            },
                        ))
                        .with_children(|right| {
                            right.spawn((
                                Text::new(L10n::skill_backpack_page(
                                    lang,
                                    current_page + 1,
                                    total_pages,
                                )),
                                TextFont {
                                    font: font.clone(),
                                    font_size: 15.0,
                                    ..default()
                                },
                                TextColor(skin::text_muted()),
                            ));

                            right.spawn((
                                Node {
                                    flex_direction: FlexDirection::Row,
                                    column_gap: Val::Px(8.0),
                                    align_items: AlignItems::Center,
                                    justify_content: JustifyContent::FlexEnd,
                                    ..default()
                                },
                            ))
                            .with_children(|pager| {
                                spawn_skill_bag_small_button(
                                    pager,
                                    asset_server,
                                    &font,
                                    L10n::equipment_prev_page(lang),
                                    SkillBagPageButton { delta: -1 },
                                );
                                spawn_skill_bag_small_button(
                                    pager,
                                    asset_server,
                                    &font,
                                    L10n::equipment_next_page(lang),
                                    SkillBagPageButton { delta: 1 },
                                );
                                spawn_skill_bag_close_button(
                                    pager,
                                    asset_server,
                                    &font,
                                    L10n::close(lang),
                                );
                            });
                        });
                });

                panel.spawn((
                    Node {
                        width: Val::Percent(100.0),
                        flex_grow: 1.0,
                        display: Display::Grid,
                        grid_auto_flow: GridAutoFlow::Column,
                        justify_content: JustifyContent::Center,
                        align_content: AlignContent::Start,
                        grid_template_columns: RepeatedGridTrack::px(5, card_w),
                        grid_template_rows: RepeatedGridTrack::px(2, card_h),
                        row_gap: Val::Px(8.0),
                        column_gap: Val::Px(8.0),
                        padding: UiRect::all(Val::Px(8.0)),
                        ..default()
                    },
                    BackgroundColor(skin::inset_tint()),
                    ImageNode::new(skin::panel(asset_server)),
                ))
                .with_children(|grid| {
                    for i in 0..SKILL_BAG_PAGE_SIZE {
                        let maybe = carried_list.get(start + i).copied();
                        match maybe {
                            Some(skill) => {
                                let def = pool.def(skill);
                                let stats = effective_skill_stats(skill, pool, runtime_stats);
                                let selected = true;
                                grid.spawn((
                                    Node {
                                        width: Val::Px(card_w),
                                        height: Val::Px(card_h),
                                        padding: UiRect::all(Val::Px(2.0)),
                                        ..default()
                                    },
                                    BackgroundColor(if selected {
                                        skin::equipped_fill()
                                    } else {
                                        skin::slot_fill()
                                    }),
                                    ImageNode::new(skin::slot(asset_server)),
                                ))
                                .with_children(|card| {
                                    card.spawn((
                                        Node {
                                            width: Val::Percent(100.0),
                                            height: Val::Percent(100.0),
                                            flex_direction: FlexDirection::Column,
                                            justify_content: JustifyContent::SpaceBetween,
                                            align_items: AlignItems::Stretch,
                                            row_gap: Val::Px(8.0),
                                            padding: UiRect::all(Val::Px(10.0)),
                                            ..default()
                                        },
                                        BackgroundColor(skin::subpanel_tint()),
                                        ImageNode::new(skin::panel(asset_server)),
                                    ))
                                    .with_children(|content| {
                                        content.spawn((
                                            Node {
                                                width: Val::Percent(100.0),
                                                justify_content: JustifyContent::Center,
                                                align_items: AlignItems::Center,
                                                padding: UiRect::horizontal(Val::Px(4.0)),
                                                ..default()
                                            },
                                        ))
                                        .with_children(|top| {
                                            top.spawn((
                                                Text::new(format!(
                                                    "{} [{}]",
                                                    L10n::skill_name(lang, skill),
                                                    L10n::skill_backpack_selected(lang)
                                                )),
                                                TextLayout::new_with_justify(Justify::Center),
                                                TextFont {
                                                    font: font.clone(),
                                                    font_size: 15.0,
                                                    ..default()
                                                },
                                                TextColor(skin::text_primary()),
                                            ));
                                        });

                                        content.spawn((
                                            Node {
                                                width: Val::Percent(100.0),
                                                flex_grow: 1.0,
                                                justify_content: JustifyContent::Center,
                                                align_items: AlignItems::Center,
                                                padding: UiRect::axes(Val::Px(6.0), Val::Px(4.0)),
                                                ..default()
                                            },
                                        ))
                                        .with_children(|middle| {
                                            middle.spawn((
                                                Text::new(format!(
                                                    "{}\n{}",
                                                    L10n::skill_card_effect(lang),
                                                    L10n::skill_effect_desc(lang, skill)
                                                )),
                                                TextLayout::new_with_justify(Justify::Center),
                                                TextFont {
                                                    font: font.clone(),
                                                    font_size: 10.0,
                                                    ..default()
                                                },
                                                TextColor(skin::text_muted()),
                                            ));
                                        });

                                        content.spawn((
                                            Node {
                                                width: Val::Percent(100.0),
                                                flex_direction: FlexDirection::Column,
                                                row_gap: Val::Px(4.0),
                                                padding: UiRect::axes(Val::Px(4.0), Val::Px(4.0)),
                                                ..default()
                                            },
                                        ))
                                        .with_children(|stats_col| {
                                            stats_col.spawn(card_stat_text(
                                                &font,
                                                format!(
                                                    "{}: {:.0}",
                                                    L10n::skill_card_damage(lang),
                                                    stats.damage
                                                ),
                                            ));
                                            stats_col.spawn(card_stat_text(
                                                &font,
                                                format!(
                                                    "{}: {:.1}s",
                                                    L10n::skill_card_cooldown(lang),
                                                    stats.cooldown
                                                ),
                                            ));
                                            stats_col.spawn(card_stat_text(
                                                &font,
                                                format!(
                                                    "{}: {}",
                                                    L10n::skill_card_rarity(lang),
                                                    L10n::skill_rarity(lang, def.rarity)
                                                ),
                                            ));
                                        });
                                    });
                                });
                            }
                            None => {
                                grid.spawn((
                                    Node {
                                        width: Val::Px(card_w),
                                        height: Val::Px(card_h),
                                        justify_content: JustifyContent::Center,
                                        align_items: AlignItems::Center,
                                        ..default()
                                    },
                                    BackgroundColor(skin::slot_fill()),
                                    ImageNode::new(skin::slot(asset_server)),
                                ))
                                .with_children(|slot| {
                                    slot.spawn((
                                        Text::new(L10n::skill_backpack_empty(lang)),
                                        TextLayout::new_with_justify(Justify::Center),
                                        TextFont {
                                            font: font.clone(),
                                            font_size: 14.0,
                                            ..default()
                                        },
                                        TextColor(skin::text_muted()),
                                    ));
                                });
                            }
                        }
                    }
                });
            });
        });
}

fn spawn_skill_bag_small_button(
    parent: &mut ChildSpawnerCommands<'_>,
    asset_server: &AssetServer,
    font: &Handle<Font>,
    label: &str,
    page_btn: SkillBagPageButton,
) {
    parent
        .spawn((
            Button,
            page_btn,
            Node {
                width: Val::Px(86.0),
                height: Val::Px(30.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(skin::button_idle()),
            ImageNode::new(skin::button_small(asset_server)),
        ))
        .with_children(|btn| {
            btn.spawn((
                Text::new(label),
                TextFont {
                    font: font.clone(),
                    font_size: 13.0,
                    ..default()
                },
                TextColor(skin::text_primary()),
            ));
        });
}

fn spawn_skill_bag_close_button(
    parent: &mut ChildSpawnerCommands<'_>,
    asset_server: &AssetServer,
    font: &Handle<Font>,
    label: &str,
) {
    parent
        .spawn((
            Button,
            SkillBagCloseButton,
            Node {
                width: Val::Px(86.0),
                height: Val::Px(30.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(skin::button_idle()),
            ImageNode::new(skin::button_small(asset_server)),
        ))
        .with_children(|btn| {
            btn.spawn((
                Text::new(label),
                TextFont {
                    font: font.clone(),
                    font_size: 13.0,
                    ..default()
                },
                TextColor(skin::text_primary()),
            ));
        });
}

fn card_stat_text(font: &Handle<Font>, text: String) -> impl Bundle {
    (
        Text::new(text),
        TextFont {
            font: font.clone(),
            font_size: 12.0,
            ..default()
        },
        TextColor(skin::text_muted()),
    )
}
