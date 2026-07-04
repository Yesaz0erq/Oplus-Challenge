use bevy::ecs::hierarchy::ChildOf;
use bevy::input::mouse::MouseButton;
use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use rand::Rng;
use std::collections::{HashMap, HashSet};

use crate::combat::{
    CombatSet, ProjectilePool, VfxPool, skill_light_wave, skill_slash,
    spawn_fireball_skill_projectile, spawn_light_wave_vfx, spawn_slash_vfx,
};
use crate::debug_tools::DebugCheats;
use crate::enemy::{Enemy, EnemyAggro, EnemyHitbox};
use crate::equipment::{EquipmentUiRoot, PlayerMemory};
use crate::health::Health;
use crate::i18n::L10n;
use crate::movement::{Player, PlayerAnimation, PlayerCamera, PlayerDash};
use crate::skills_pool::{SkillId, SkillPool};
use crate::state::GameState;
use crate::ui::EscBlockingUi;
use crate::ui::pause_menu::SuppressPauseMenuOnce;
use crate::ui::skin;
use crate::ui::types::GameSettings;

const MAX_SKILL_CARDS: usize = 3;
const SKILL_HUD_SCALE: f32 = 1.5;
const SKILL_HUD_CARD_WIDTH: f32 = 106.0 * SKILL_HUD_SCALE;
const SKILL_HUD_CARD_HEIGHT: f32 = 130.0 * SKILL_HUD_SCALE;
const SKILL_HUD_DECK_WIDTH: f32 = SKILL_HUD_CARD_WIDTH;
const SKILL_HUD_DECK_HEIGHT: f32 = SKILL_HUD_CARD_HEIGHT;
const SKILL_HUD_DECK_GAP: f32 = 14.0 * SKILL_HUD_SCALE;
const SKILL_HUD_DECK_STACK_SPREAD: f32 = 9.0 * SKILL_HUD_SCALE;
const SKILL_HUD_CARD_ORIGIN_X: f32 =
    SKILL_HUD_DECK_WIDTH + SKILL_HUD_DECK_STACK_SPREAD + SKILL_HUD_DECK_GAP;
/// Horizontal advance between adjacent cards. Smaller than the card width so
/// the hand overlaps (occludes) like a real fan.
const SKILL_HUD_CARD_STEP: f32 = 66.0 * SKILL_HUD_SCALE;
/// Vertical lift applied to a hovered card.
const SKILL_HUD_CARD_HOVER_LIFT: f32 = 18.0 * SKILL_HUD_SCALE;
const SKILL_FACE_DAMAGE_CENTER_X: f32 = 0.378;
const SKILL_FACE_COOLDOWN_CENTER_X: f32 = 0.79;
const SKILL_FACE_VALUE_CENTER_Y: f32 = 0.702;
const SKILL_PARSE_FACE_VALUE_CENTER_Y: f32 = 0.64;
const SKILL_HUD_FACE_DAMAGE_WIDTH: f32 = 28.0 * SKILL_HUD_SCALE;
const SKILL_HUD_FACE_COOLDOWN_WIDTH: f32 = 42.0 * SKILL_HUD_SCALE;
const SKILL_HUD_FACE_DAMAGE_LEFT: f32 =
    SKILL_HUD_CARD_WIDTH * SKILL_FACE_DAMAGE_CENTER_X - SKILL_HUD_FACE_DAMAGE_WIDTH * 0.5;
const SKILL_HUD_FACE_COOLDOWN_LEFT: f32 =
    SKILL_HUD_CARD_WIDTH * SKILL_FACE_COOLDOWN_CENTER_X - SKILL_HUD_FACE_COOLDOWN_WIDTH * 0.5;
const SKILL_HUD_FACE_VALUE_TOP: f32 = 82.0 * SKILL_HUD_SCALE;
const SKILL_BAG_PAGE_SIZE: usize = 10;
const SKILL_BAG_GRID_COLUMNS: usize = 5;
const SKILL_BAG_CARD_DEAL_TIME: f32 = 0.36;
const SKILL_BAG_CARD_DEAL_STAGGER: f32 = 0.045;
const SKILL_BAG_CARD_HOVER_LIFT: f32 = 22.0;
const SKILL_PARSE_COOLDOWN: f32 = 60.0;

#[derive(Component)]
struct SkillUiRoot;

#[derive(Component)]
struct SkillCard {
    slot_index: usize,
    skill: Option<SkillId>,
}

#[derive(Component, Clone)]
struct SkillCardImages {
    fallback: Handle<Image>,
    back_green: Handle<Image>,
    back_blue: Handle<Image>,
    back_red: Handle<Image>,
    back_gold: Handle<Image>,
    slash_green: Handle<Image>,
    slash_blue: Handle<Image>,
    slash_red: Handle<Image>,
    slash_gold: Handle<Image>,
    fireball_green: Handle<Image>,
    fireball_blue: Handle<Image>,
    fireball_red: Handle<Image>,
    fireball_gold: Handle<Image>,
}

#[derive(Component)]
struct SkillCardFallbackText {
    slot_index: usize,
    color: Color,
}

#[derive(Component)]
struct SkillCardDamageValueText {
    slot_index: usize,
}

#[derive(Component)]
struct SkillCardCooldownValueText {
    slot_index: usize,
}

/// Drives the Slay-the-Spire style draw / play / hover motion for one HUD card.
#[derive(Component)]
struct CardAnim {
    phase: CardPhase,
    timer: Timer,
    prev_skill: Option<SkillId>,
    /// True when the change should fly in from the deck (a fresh draw) rather
    /// than a small in-place refresh.
    from_deck: bool,
}

impl Default for CardAnim {
    fn default() -> Self {
        Self {
            phase: CardPhase::Idle,
            timer: Timer::from_seconds(0.0, TimerMode::Once),
            prev_skill: None,
            from_deck: false,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum CardPhase {
    Idle,
    /// Animating into the hand (fresh draw or refresh after a play).
    Drawing,
    /// Played card briefly fades/lifts before the slot goes empty.
    Playing,
}

/// Decorative draw-pile stack drawn to the right of the hand.
#[derive(Component)]
struct SkillDeckGlyph;

#[derive(Component)]
struct SkillDeckLayer {
    depth: usize,
}

#[derive(Component)]
struct SkillDeckAnim {
    age: f32,
}

#[derive(Component, Clone)]
struct SkillDeckImages {
    green: Handle<Image>,
    blue: Handle<Image>,
    red: Handle<Image>,
    gold: Handle<Image>,
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
pub struct SkillBagUiRoot;

#[derive(Component)]
struct SkillBagPageButton {
    delta: i32,
}

#[derive(Component)]
struct SkillBagCloseButton;

#[derive(Component)]
struct SkillBagCard {
    slot_index: usize,
    skill: SkillId,
}

#[derive(Component)]
struct SkillBagCardAnim {
    age: f32,
    hover: f32,
    seed: f32,
}

impl SkillBagCardAnim {
    fn new(slot_index: usize) -> Self {
        Self {
            age: 0.0,
            hover: 0.0,
            seed: slot_index as f32 * 1.37,
        }
    }
}

#[derive(Component)]
struct SkillBagDetailRoot;

#[derive(Component)]
struct SkillBagDetailBackdrop;

#[derive(Component)]
struct SkillBagDetailCloseButton;

#[derive(Component)]
struct SkillBagDetailCardAnim {
    age: f32,
}

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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SkillCardFaceTier {
    Green,
    Blue,
    Red,
    Gold,
}

impl SkillCardFaceTier {
    fn from_damage_ratio(ratio: f32) -> Self {
        if ratio < 0.90 {
            Self::Green
        } else if ratio <= 1.10 {
            Self::Blue
        } else if ratio < 1.45 {
            Self::Red
        } else {
            Self::Gold
        }
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
                    animate_skill_cards,
                    animate_skill_deck_glyph,
                    animate_skill_deck_button,
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
                    handle_skill_deck_clicks,
                    process_skill_bag_open_request,
                    handle_skill_bag_page_buttons,
                    handle_skill_bag_close_button,
                    handle_skill_bag_card_clicks,
                    handle_skill_bag_detail_close_button,
                    rebuild_skill_bag_ui_when_dirty,
                    animate_skill_bag_cards,
                    animate_skill_bag_detail_card,
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
    let font = skin::ui_font(&asset_server, lang);
    let hud_slot = skin::hud_slot(&asset_server);
    let slash_green: Handle<Image> = asset_server.load("ui/cards/slash_green.png");
    let slash_blue: Handle<Image> = asset_server.load("ui/cards/slash_blue.png");
    let slash_red: Handle<Image> = asset_server.load("ui/cards/slash_red.png");
    let slash_gold: Handle<Image> = asset_server.load("ui/cards/slash_gold.png");
    let fireball_green: Handle<Image> = asset_server.load("ui/cards/fireball_green.png");
    let fireball_blue: Handle<Image> = asset_server.load("ui/cards/fireball_blue.png");
    let fireball_red: Handle<Image> = asset_server.load("ui/cards/fireball_red.png");
    let fireball_gold: Handle<Image> = asset_server.load("ui/cards/fireball_gold.png");
    let back_green: Handle<Image> = asset_server.load("ui/cards/back_green.png");
    let back_blue: Handle<Image> = asset_server.load("ui/cards/back_blue.png");
    let back_red: Handle<Image> = asset_server.load("ui/cards/back_red.png");
    let back_gold: Handle<Image> = asset_server.load("ui/cards/back_gold.png");
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
                font: font.clone().into(),
                font_size: FontSize::from(18.0),
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
                font: font.clone().into(),
                font_size: FontSize::from(14.0),
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
                font: font.clone().into(),
                font_size: FontSize::from(14.0),
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

        let hand_width =
            SKILL_HUD_CARD_STEP * (MAX_SKILL_CARDS.saturating_sub(1)) as f32 + SKILL_HUD_CARD_WIDTH;
        let hand_area_width = SKILL_HUD_CARD_ORIGIN_X + hand_width;
        parent
            .spawn((Node {
                position_type: PositionType::Absolute,
                left: Val::Px(20.0),
                bottom: Val::Px(18.0),
                width: Val::Px(hand_area_width),
                height: Val::Px(
                    SKILL_HUD_CARD_HEIGHT
                        + SKILL_HUD_CARD_HOVER_LIFT
                        + SKILL_HUD_DECK_STACK_SPREAD
                        + 10.0,
                ),
                ..default()
            },))
            .with_children(|hand| {
                hand.spawn((
                    SkillDeckGlyph,
                    Button,
                    Interaction::default(),
                    Node {
                        position_type: PositionType::Absolute,
                        left: Val::Px(0.0),
                        bottom: Val::Px(0.0),
                        width: Val::Px(SKILL_HUD_DECK_WIDTH + SKILL_HUD_DECK_STACK_SPREAD),
                        height: Val::Px(SKILL_HUD_DECK_HEIGHT + SKILL_HUD_DECK_STACK_SPREAD),
                        ..default()
                    },
                    BackgroundColor(Color::NONE),
                    UiTransform::IDENTITY,
                    ZIndex(120),
                ))
                .with_children(|deck| {
                    for depth in 0..4 {
                        let offset = depth as f32 * SKILL_HUD_DECK_STACK_SPREAD / 3.0;
                        deck.spawn((
                            SkillDeckLayer { depth },
                            SkillDeckAnim { age: 0.0 },
                            SkillDeckImages {
                                green: back_green.clone(),
                                blue: back_blue.clone(),
                                red: back_red.clone(),
                                gold: back_gold.clone(),
                            },
                            Node {
                                position_type: PositionType::Absolute,
                                left: Val::Px(offset),
                                bottom: Val::Px(offset),
                                width: Val::Px(SKILL_HUD_DECK_WIDTH),
                                height: Val::Px(SKILL_HUD_DECK_HEIGHT),
                                ..default()
                            },
                            ImageNode::new(back_blue.clone()),
                            BackgroundColor(Color::NONE),
                            UiTransform::IDENTITY,
                            ZIndex(depth as i32),
                        ));
                    }
                });

                for (i, skill) in initial_slots.iter().enumerate().take(MAX_SKILL_CARDS) {
                    let base_left = SKILL_HUD_CARD_ORIGIN_X + i as f32 * SKILL_HUD_CARD_STEP;
                    hand.spawn((
                        SkillCard {
                            slot_index: i,
                            skill: *skill,
                        },
                        CardAnim::default(),
                        SkillCardImages {
                            fallback: hud_slot.clone(),
                            back_green: back_green.clone(),
                            back_blue: back_blue.clone(),
                            back_red: back_red.clone(),
                            back_gold: back_gold.clone(),
                            slash_green: slash_green.clone(),
                            slash_blue: slash_blue.clone(),
                            slash_red: slash_red.clone(),
                            slash_gold: slash_gold.clone(),
                            fireball_green: fireball_green.clone(),
                            fireball_blue: fireball_blue.clone(),
                            fireball_red: fireball_red.clone(),
                            fireball_gold: fireball_gold.clone(),
                        },
                        Button,
                        Interaction::default(),
                        Node {
                            position_type: PositionType::Absolute,
                            left: Val::Px(base_left),
                            bottom: Val::Px(0.0),
                            width: Val::Px(SKILL_HUD_CARD_WIDTH),
                            height: Val::Px(SKILL_HUD_CARD_HEIGHT),
                            padding: UiRect::axes(
                                Val::Px(8.0 * SKILL_HUD_SCALE),
                                Val::Px(10.0 * SKILL_HUD_SCALE),
                            ),
                            flex_direction: FlexDirection::Column,
                            justify_content: JustifyContent::SpaceBetween,
                            align_items: AlignItems::Stretch,
                            ..default()
                        },
                        BackgroundColor(Color::NONE),
                        BorderColor::all(Color::NONE),
                        ImageNode::new(hud_slot.clone()),
                        UiTransform::IDENTITY,
                        ZIndex(i as i32),
                    ))
                    .with_children(|card| {
                        card.spawn((Node {
                            width: Val::Percent(100.0),
                            justify_content: JustifyContent::SpaceBetween,
                            align_items: AlignItems::Center,
                            ..default()
                        },))
                            .with_children(|header| {
                                header.spawn((
                                    SkillCardFallbackText {
                                        slot_index: i,
                                        color: skin::text_accent(),
                                    },
                                    Text::new((i + 1).to_string()),
                                    TextFont {
                                        font: font.clone().into(),
                                        font_size: FontSize::from(15.0 * SKILL_HUD_SCALE),
                                        ..default()
                                    },
                                    TextColor(skin::text_accent()),
                                ));
                                header.spawn((
                                    SkillCardFallbackText {
                                        slot_index: i,
                                        color: skin::text_dim(),
                                    },
                                    Text::new("SKILL"),
                                    TextFont {
                                        font: font.clone().into(),
                                        font_size: FontSize::from(8.0 * SKILL_HUD_SCALE),
                                        ..default()
                                    },
                                    TextColor(skin::text_dim()),
                                ));
                            });

                        card.spawn((Node {
                            width: Val::Percent(100.0),
                            flex_grow: 1.0,
                            justify_content: JustifyContent::Center,
                            align_items: AlignItems::Center,
                            padding: UiRect::axes(
                                Val::Px(4.0 * SKILL_HUD_SCALE),
                                Val::Px(6.0 * SKILL_HUD_SCALE),
                            ),
                            ..default()
                        },))
                            .with_children(|body| {
                                body.spawn((
                                    SkillNameText { slot_index: i },
                                    SkillCardFallbackText {
                                        slot_index: i,
                                        color: skin::text_primary(),
                                    },
                                    Text::new(""),
                                    TextLayout::justify(Justify::Center),
                                    TextFont {
                                        font: font.clone().into(),
                                        font_size: FontSize::from(14.0 * SKILL_HUD_SCALE),
                                        ..default()
                                    },
                                    TextColor(skin::text_primary()),
                                ));
                            });

                        card.spawn((
                            SkillCooldownText { slot_index: i },
                            SkillCardFallbackText {
                                slot_index: i,
                                color: skin::text_muted(),
                            },
                            Text::new(""),
                            TextLayout::justify(Justify::Center),
                            TextFont {
                                font: font.clone().into(),
                                font_size: FontSize::from(10.0 * SKILL_HUD_SCALE),
                                ..default()
                            },
                            TextColor(skin::text_muted()),
                            Node {
                                width: Val::Percent(100.0),
                                ..default()
                            },
                        ));

                        card.spawn((
                            SkillCardDamageValueText { slot_index: i },
                            Text::new(""),
                            TextLayout::justify(Justify::Center),
                            TextFont {
                                font: font.clone().into(),
                                font_size: FontSize::from(13.0 * SKILL_HUD_SCALE),
                                ..default()
                            },
                            TextColor(Color::srgb(0.70, 0.08, 0.10)),
                            Node {
                                position_type: PositionType::Absolute,
                                left: Val::Px(SKILL_HUD_FACE_DAMAGE_LEFT),
                                top: Val::Px(SKILL_HUD_FACE_VALUE_TOP),
                                width: Val::Px(SKILL_HUD_FACE_DAMAGE_WIDTH),
                                height: Val::Px(18.0 * SKILL_HUD_SCALE),
                                justify_content: JustifyContent::Center,
                                align_items: AlignItems::Center,
                                ..default()
                            },
                            ZIndex(50),
                        ));

                        card.spawn((
                            SkillCardCooldownValueText { slot_index: i },
                            Text::new(""),
                            TextLayout::justify(Justify::Center),
                            TextFont {
                                font: font.clone().into(),
                                font_size: FontSize::from(10.5 * SKILL_HUD_SCALE),
                                ..default()
                            },
                            TextColor(Color::srgb(0.04, 0.04, 0.04)),
                            Node {
                                position_type: PositionType::Absolute,
                                left: Val::Px(SKILL_HUD_FACE_COOLDOWN_LEFT),
                                top: Val::Px(SKILL_HUD_FACE_VALUE_TOP),
                                width: Val::Px(SKILL_HUD_FACE_COOLDOWN_WIDTH),
                                height: Val::Px(18.0 * SKILL_HUD_SCALE),
                                justify_content: JustifyContent::Center,
                                align_items: AlignItems::Center,
                                ..default()
                            },
                            ZIndex(50),
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

/// Slay-the-Spire style hand motion: cards deal in from the draw pile with an
/// overshoot pop, lift and straighten when hovered (rising above their
/// neighbours), refresh when the hand shifts, and fade out when played.
fn animate_skill_cards(
    time: Res<Time>,
    pool: Res<SkillPool>,
    runtime_stats: Res<SkillRuntimeStats>,
    mut cards: Query<(
        &SkillCard,
        &SkillCardImages,
        &mut CardAnim,
        &mut UiTransform,
        &mut ZIndex,
        &Interaction,
        &mut ImageNode,
        &mut BorderColor,
        &mut BackgroundColor,
    )>,
) {
    let dt = time.delta();
    let dt_s = time.delta_secs();
    let deck_x = SKILL_HUD_DECK_STACK_SPREAD - SKILL_HUD_CARD_ORIGIN_X;
    let center = (MAX_SKILL_CARDS as f32 - 1.0) * 0.5;
    let smooth = 1.0 - (-18.0 * dt_s).exp();

    let read_px = |v: Val| match v {
        Val::Px(px) => px,
        _ => 0.0,
    };

    for (
        card,
        card_images,
        mut anim,
        mut transform,
        mut z,
        interaction,
        mut image,
        mut border,
        mut fill,
    ) in &mut cards
    {
        let previous_skill = anim.prev_skill;
        let face_tier = card
            .skill
            .map(|skill| skill_card_face_tier(skill, &pool, &runtime_stats))
            .unwrap_or(SkillCardFaceTier::Blue);
        let previous_tier = previous_skill
            .map(|skill| skill_card_face_tier(skill, &pool, &runtime_stats))
            .unwrap_or(face_tier);
        let face_image = match (card.skill, face_tier) {
            (Some(SkillId::Slash), SkillCardFaceTier::Green) => card_images.slash_green.clone(),
            (Some(SkillId::Slash), SkillCardFaceTier::Blue) => card_images.slash_blue.clone(),
            (Some(SkillId::Slash), SkillCardFaceTier::Red) => card_images.slash_red.clone(),
            (Some(SkillId::Slash), SkillCardFaceTier::Gold) => card_images.slash_gold.clone(),
            (Some(SkillId::Fireball), SkillCardFaceTier::Green) => {
                card_images.fireball_green.clone()
            }
            (Some(SkillId::Fireball), SkillCardFaceTier::Blue) => card_images.fireball_blue.clone(),
            (Some(SkillId::Fireball), SkillCardFaceTier::Red) => card_images.fireball_red.clone(),
            (Some(SkillId::Fireball), SkillCardFaceTier::Gold) => card_images.fireball_gold.clone(),
            _ => card_images.fallback.clone(),
        };
        let previous_face_image = match (previous_skill, previous_tier) {
            (Some(SkillId::Slash), SkillCardFaceTier::Green) => card_images.slash_green.clone(),
            (Some(SkillId::Slash), SkillCardFaceTier::Blue) => card_images.slash_blue.clone(),
            (Some(SkillId::Slash), SkillCardFaceTier::Red) => card_images.slash_red.clone(),
            (Some(SkillId::Slash), SkillCardFaceTier::Gold) => card_images.slash_gold.clone(),
            (Some(SkillId::Fireball), SkillCardFaceTier::Green) => {
                card_images.fireball_green.clone()
            }
            (Some(SkillId::Fireball), SkillCardFaceTier::Blue) => card_images.fireball_blue.clone(),
            (Some(SkillId::Fireball), SkillCardFaceTier::Red) => card_images.fireball_red.clone(),
            (Some(SkillId::Fireball), SkillCardFaceTier::Gold) => card_images.fireball_gold.clone(),
            _ => card_images.fallback.clone(),
        };
        let back_image = match face_tier {
            SkillCardFaceTier::Green => card_images.back_green.clone(),
            SkillCardFaceTier::Blue => card_images.back_blue.clone(),
            SkillCardFaceTier::Red => card_images.back_red.clone(),
            SkillCardFaceTier::Gold => card_images.back_gold.clone(),
        };

        // Detect a content change and kick off the matching animation.
        if card.skill != anim.prev_skill {
            match (anim.prev_skill, card.skill) {
                (None, Some(_)) => {
                    anim.phase = CardPhase::Drawing;
                    anim.from_deck = true;
                    anim.timer = Timer::from_seconds(0.62, TimerMode::Once);
                }
                (Some(_), Some(_)) => {
                    anim.phase = CardPhase::Drawing;
                    anim.from_deck = false;
                    anim.timer = Timer::from_seconds(0.22, TimerMode::Once);
                }
                (Some(_), None) => {
                    anim.phase = CardPhase::Playing;
                    anim.from_deck = false;
                    anim.timer = Timer::from_seconds(0.28, TimerMode::Once);
                }
                (None, None) => {}
            }
            anim.prev_skill = card.skill;
        }

        anim.timer.tick(dt);

        let hovered = card.skill.is_some()
            && matches!(interaction, Interaction::Hovered | Interaction::Pressed);

        // Resting fan pose for this slot.
        let rel = card.slot_index as f32 - center;
        let base_rot = rel * 0.05;
        let base_y = rel.abs() * 3.0;
        let (rest_x, rest_y) = (
            0.0,
            base_y
                + if hovered {
                    -SKILL_HUD_CARD_HOVER_LIFT
                } else {
                    0.0
                },
        );
        let rest_rot = if hovered { 0.0 } else { base_rot };
        let rest_scale = if hovered { 1.12 } else { 1.0 };

        let mut alpha = if card.skill.is_some() { 1.0 } else { 0.0 };
        let mut target_z = if hovered { 100 } else { card.slot_index as i32 };

        match anim.phase {
            CardPhase::Drawing => {
                let p = crate::ui::anim::Ease::OutBack.apply(anim.timer.fraction());
                let lin = anim.timer.fraction();
                let (start_x, start_y, start_scale, start_rot) = if anim.from_deck {
                    (
                        deck_x - card.slot_index as f32 * SKILL_HUD_CARD_STEP,
                        SKILL_HUD_DECK_STACK_SPREAD,
                        0.70,
                        -0.28,
                    )
                } else {
                    (12.0, 10.0, 0.86, 0.16)
                };
                transform.translation = Val2::px(
                    start_x + (rest_x - start_x) * p,
                    start_y + (rest_y - start_y) * p,
                );
                let motion_scale = start_scale + (rest_scale - start_scale) * p;
                let flip_t = if anim.from_deck {
                    ((lin - 0.16) / 0.62).clamp(0.0, 1.0)
                } else {
                    1.0
                };
                let flip_squash = (1.0 - (std::f32::consts::PI * flip_t).sin() * 0.88).max(0.12);
                transform.scale = Vec2::new(motion_scale * flip_squash, motion_scale);
                transform.rotation = Rot2::radians(start_rot + (rest_rot - start_rot) * p);
                alpha = lin.clamp(0.0, 1.0);
                target_z = 90;
                image.image = if anim.from_deck && flip_t < 0.5 {
                    back_image.clone()
                } else {
                    face_image.clone()
                };
                if anim.timer.is_finished() {
                    anim.phase = CardPhase::Idle;
                }
            }
            CardPhase::Playing => {
                let p = anim.timer.fraction();
                let eased = crate::ui::anim::Ease::OutCubic.apply(p);
                let throw_dir = if rel >= 0.0 { 1.0 } else { -1.0 };
                transform.translation =
                    Val2::px(rest_x + 36.0 * throw_dir * eased, rest_y - 52.0 * eased);
                transform.scale = Vec2::splat(
                    (1.0 + 0.16 * (1.0 - (2.0 * p - 1.0).abs())) * (1.0 - 0.22 * eased),
                );
                transform.rotation = Rot2::radians(rest_rot + throw_dir * 0.42 * eased);
                alpha = (1.0 - p).clamp(0.0, 1.0);
                target_z = 90;
                image.image = previous_face_image.clone();
                if anim.timer.is_finished() {
                    anim.phase = CardPhase::Idle;
                }
            }
            CardPhase::Idle => {
                // Smoothly chase the resting pose (handles hover in/out).
                let cur_x = read_px(transform.translation.x);
                let cur_y = read_px(transform.translation.y);
                transform.translation = Val2::px(
                    cur_x + (rest_x - cur_x) * smooth,
                    cur_y + (rest_y - cur_y) * smooth,
                );
                let cur_scale = transform.scale.x;
                transform.scale = Vec2::splat(cur_scale + (rest_scale - cur_scale) * smooth);
                let cur_rot = transform.rotation.as_radians();
                transform.rotation = Rot2::radians(cur_rot + (rest_rot - cur_rot) * smooth);
                image.image = face_image.clone();
            }
        }

        image.color = image.color.with_alpha(alpha);
        *border = BorderColor::all(Color::NONE);
        fill.0 = Color::NONE;
        *z = ZIndex(target_z);
    }
}

fn animate_skill_deck_glyph(
    time: Res<Time>,
    carried: Res<CarriedSkills>,
    cooldowns: Res<SkillCooldowns>,
    pool: Res<SkillPool>,
    runtime_stats: Res<SkillRuntimeStats>,
    mut decks: Query<(
        &SkillDeckLayer,
        &mut SkillDeckAnim,
        &SkillDeckImages,
        &mut UiTransform,
        &mut ImageNode,
    )>,
) {
    let dt = time.delta_secs();
    let now = time.elapsed_secs();
    let tier = next_cooling_skill_tier(&carried, &cooldowns, &pool, &runtime_stats);

    for (layer, mut anim, images, mut transform, mut image) in &mut decks {
        anim.age += dt;
        let depth = layer.depth as f32;
        let pulse = (now * 2.4 + depth * 0.35).sin() * 0.5 + 0.5;
        let tilt = (now * 1.7 + depth * 0.42).sin();

        image.image = match tier {
            SkillCardFaceTier::Green => images.green.clone(),
            SkillCardFaceTier::Blue => images.blue.clone(),
            SkillCardFaceTier::Red => images.red.clone(),
            SkillCardFaceTier::Gold => images.gold.clone(),
        };
        transform.translation = Val2::px(depth * 0.45, -2.0 * pulse + depth * 0.35);
        transform.scale = Vec2::splat(1.0 + 0.018 * pulse - depth * 0.004);
        transform.rotation = Rot2::radians(tilt * 0.018 + (depth - 1.5) * 0.012);
    }
}

fn handle_skill_deck_clicks(
    mut interactions: Query<&Interaction, (Changed<Interaction>, With<SkillDeckGlyph>)>,
    mut request_open_bag: ResMut<SkillBagOpenRequest>,
) {
    for interaction in &mut interactions {
        if matches!(*interaction, Interaction::Pressed) {
            request_open_bag.0 = true;
        }
    }
}

fn animate_skill_deck_button(
    time: Res<Time>,
    mut decks: Query<(&Interaction, &mut UiTransform), With<SkillDeckGlyph>>,
) {
    let now = time.elapsed_secs();
    for (interaction, mut transform) in &mut decks {
        let idle = (now * 1.9).sin() * 0.5 + 0.5;
        let (lift, scale, rot) = match *interaction {
            Interaction::Pressed => (-5.0, 0.96, -0.035),
            Interaction::Hovered => (-10.0, 1.07, 0.025),
            Interaction::None => (-2.0 * idle, 1.0 + 0.018 * idle, 0.0),
        };
        transform.translation = Val2::px(0.0, lift);
        transform.scale = Vec2::splat(scale);
        transform.rotation = Rot2::radians(rot);
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

fn skill_damage_ratio(skill: SkillId, pool: &SkillPool, stats: SkillNumericStats) -> f32 {
    let base = pool.def(skill).damage;
    if base <= f32::EPSILON {
        1.0
    } else {
        stats.damage / base
    }
}

fn skill_card_face_tier(
    skill: SkillId,
    pool: &SkillPool,
    runtime: &SkillRuntimeStats,
) -> SkillCardFaceTier {
    let Some(stats) = runtime.0.get(&skill).copied() else {
        return SkillCardFaceTier::Blue;
    };
    SkillCardFaceTier::from_damage_ratio(skill_damage_ratio(skill, pool, stats))
}

fn next_cooling_skill_tier(
    carried: &CarriedSkills,
    cooldowns: &SkillCooldowns,
    pool: &SkillPool,
    runtime: &SkillRuntimeStats,
) -> SkillCardFaceTier {
    let next = carried
        .slots
        .iter()
        .copied()
        .flatten()
        .filter(|skill| *skill != SkillId::Dash)
        .filter_map(|skill| {
            let remaining = cooldowns.remaining(skill);
            (remaining > 0.0).then_some((skill, remaining))
        })
        .min_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
        .map(|(skill, _)| skill);

    next.map(|skill| skill_card_face_tier(skill, pool, runtime))
        .unwrap_or(SkillCardFaceTier::Blue)
}

fn skill_has_card_face(skill: SkillId) -> bool {
    matches!(skill, SkillId::Slash | SkillId::Fireball)
}

fn format_card_damage_value(value: f32) -> String {
    if (value - value.round()).abs() < 0.05 {
        format!("{:.0}", value)
    } else {
        format!("{:.1}", value)
    }
}

fn format_card_cooldown_value(value: f32, lang: crate::i18n::Language) -> String {
    let suffix = if lang == crate::i18n::Language::ZhCn {
        "秒"
    } else {
        "s"
    };
    if (value - value.round()).abs() < 0.05 {
        format!("{:.0}{suffix}", value)
    } else {
        format!("{:.1}{suffix}", value)
    }
}

fn roll_stolen_skill_stats(base: SkillNumericStats) -> SkillNumericStats {
    let mut rng = rand::thread_rng();
    let potency = if rng.gen_bool(0.05) {
        1.50
    } else {
        rng.gen_range(0.75..=1.25)
    };
    SkillNumericStats {
        damage: ((base.damage * potency) * 10.0).round() / 10.0,
        cooldown: ((base.cooldown / potency) * 10.0).round() / 10.0,
    }
}

fn spawn_skill_parse_popup_ui(
    commands: &mut Commands,
    asset_server: &AssetServer,
    pool: &SkillPool,
    lang: crate::i18n::Language,
    offer: SkillParseOffer,
    carried_count: usize,
    capacity: usize,
) {
    let font = skin::ui_font(asset_server, lang);
    let skill_name = L10n::skill_name(lang, offer.skill);
    let is_new_skill = offer.current.is_none();

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

    // Capacity warning for new skills
    let cap_warning = if is_new_skill {
        let mut w = format!(
            "{}: {}/{}",
            L10n::skill_backpack_capacity_label(lang),
            carried_count,
            capacity
        );
        if carried_count >= capacity {
            w.push_str(&format!("\n{}", L10n::skill_parse_bag_full(lang)));
        }
        Some(w)
    } else {
        None
    };

    // Face images
    let (old_face, new_face) = parse_face_images(asset_server, pool, offer);

    let card_w = 190.0;
    let card_h = 260.0;

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
                    max_width: Val::Percent(90.0),
                    padding: UiRect::all(Val::Px(18.0)),
                    flex_direction: FlexDirection::Column,
                    row_gap: Val::Px(14.0),
                    align_items: AlignItems::Center,
                    border: UiRect::all(Val::Px(1.5)),
                    border_radius: BorderRadius::all(Val::Px(skin::PANEL_RADIUS)),
                    ..default()
                },
                BackgroundColor(skin::panel_tint().with_alpha(0.88)),
                BorderColor::all(skin::border_soft()),
                UiTransform::IDENTITY,
                crate::ui::anim::UiTween::pop_in(0.28, 0.94),
            ))
            .with_children(|panel| {
                // Title
                panel.spawn((
                    Text::new(L10n::skill_parse_popup_title(lang)),
                    TextFont {
                        font: font.clone().into(),
                        font_size: FontSize::from(skin::FONT_HEADING),
                        ..default()
                    },
                    TextColor(skin::text_accent()),
                ));
                // Skill name
                panel.spawn((
                    Text::new(skill_name),
                    TextFont {
                        font: font.clone().into(),
                        font_size: FontSize::from(18.0),
                        ..default()
                    },
                    TextColor(skin::text_primary()),
                ));
                // Capacity warning
                if let Some(w) = &cap_warning {
                    panel.spawn((
                        Text::new(w.clone()),
                        TextFont {
                            font: font.clone().into(),
                            font_size: FontSize::from(13.0),
                            ..default()
                        },
                        TextColor(skin::text_muted()),
                    ));
                }
                // Card comparison
                panel
                    .spawn((Node {
                        flex_direction: FlexDirection::Row,
                        justify_content: JustifyContent::Center,
                        column_gap: Val::Px(16.0),
                        align_items: AlignItems::Center,
                        ..default()
                    },))
                    .with_children(|cards| {
                        if let Some(old) = offer.current {
                            spawn_parse_skill_card(
                                cards,
                                &font,
                                lang,
                                card_w,
                                card_h,
                                old_face.clone(),
                                L10n::skill_parse_old_values(lang),
                                old,
                            );
                        }
                        spawn_parse_skill_card(
                            cards,
                            &font,
                            lang,
                            card_w,
                            card_h,
                            new_face,
                            L10n::skill_parse_new_values(lang),
                            offer.candidate,
                        );
                    });
                // Buttons
                panel
                    .spawn((Node {
                        flex_direction: FlexDirection::Row,
                        justify_content: JustifyContent::Center,
                        column_gap: Val::Px(10.0),
                        ..default()
                    },))
                    .with_children(|buttons| {
                        buttons
                            .spawn((
                                Button,
                                SkillParsePopupButton {
                                    action: SkillParsePopupAction::Reject,
                                },
                                Node {
                                    width: Val::Px(120.0),
                                    height: Val::Px(38.0),
                                    justify_content: JustifyContent::Center,
                                    align_items: AlignItems::Center,
                                    border: UiRect::all(Val::Px(1.5)),
                                    border_radius: BorderRadius::all(Val::Px(skin::BUTTON_RADIUS)),
                                    ..default()
                                },
                                BackgroundColor(skin::button_idle()),
                                BorderColor::all(skin::border_soft()),
                                skin::shadow_card(),
                                UiTransform::IDENTITY,
                                crate::ui::anim::HoverMotion::default(),
                            ))
                            .with_children(|btn| {
                                btn.spawn((
                                    Text::new(reject_label),
                                    TextFont {
                                        font: font.clone().into(),
                                        font_size: FontSize::from(skin::FONT_BODY),
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
                                    height: Val::Px(38.0),
                                    justify_content: JustifyContent::Center,
                                    align_items: AlignItems::Center,
                                    border: UiRect::all(Val::Px(1.5)),
                                    border_radius: BorderRadius::all(Val::Px(skin::BUTTON_RADIUS)),
                                    ..default()
                                },
                                BackgroundColor(skin::button_primary()),
                                BorderColor::all(skin::border_soft()),
                                skin::shadow_card(),
                                UiTransform::IDENTITY,
                                crate::ui::anim::HoverMotion::default(),
                            ))
                            .with_children(|btn| {
                                btn.spawn((
                                    Text::new(action_label),
                                    TextFont {
                                        font: font.clone().into(),
                                        font_size: FontSize::from(skin::FONT_BODY),
                                        ..default()
                                    },
                                    TextColor(skin::text_primary()),
                                ));
                            });
                    });
            });
        });
}

fn parse_face_images(
    asset_server: &AssetServer,
    pool: &SkillPool,
    offer: SkillParseOffer,
) -> (Handle<Image>, Handle<Image>) {
    let base_damage = pool.def(offer.skill).damage;
    let old_tier = offer
        .current
        .map(|s| SkillCardFaceTier::from_damage_ratio(s.damage / base_damage.max(0.01)))
        .unwrap_or(SkillCardFaceTier::Blue);
    let new_tier =
        SkillCardFaceTier::from_damage_ratio(offer.candidate.damage / base_damage.max(0.01));
    (
        face_image_for_skill(asset_server, offer.skill, old_tier),
        face_image_for_skill(asset_server, offer.skill, new_tier),
    )
}

fn face_image_for_skill(
    asset_server: &AssetServer,
    skill: SkillId,
    tier: SkillCardFaceTier,
) -> Handle<Image> {
    match (skill, tier) {
        (SkillId::Slash, SkillCardFaceTier::Green) => asset_server.load("ui/cards/slash_green.png"),
        (SkillId::Slash, SkillCardFaceTier::Blue) => asset_server.load("ui/cards/slash_blue.png"),
        (SkillId::Slash, SkillCardFaceTier::Red) => asset_server.load("ui/cards/slash_red.png"),
        (SkillId::Slash, SkillCardFaceTier::Gold) => asset_server.load("ui/cards/slash_gold.png"),
        (SkillId::Fireball, SkillCardFaceTier::Green) => {
            asset_server.load("ui/cards/fireball_green.png")
        }
        (SkillId::Fireball, SkillCardFaceTier::Blue) => {
            asset_server.load("ui/cards/fireball_blue.png")
        }
        (SkillId::Fireball, SkillCardFaceTier::Red) => {
            asset_server.load("ui/cards/fireball_red.png")
        }
        (SkillId::Fireball, SkillCardFaceTier::Gold) => {
            asset_server.load("ui/cards/fireball_gold.png")
        }
        _ => asset_server.load("ui/hud_slot.png"),
    }
}

fn optional_face_image_for_skill(
    asset_server: &AssetServer,
    skill: SkillId,
    tier: SkillCardFaceTier,
) -> Option<Handle<Image>> {
    matches!(skill, SkillId::Slash | SkillId::Fireball)
        .then(|| face_image_for_skill(asset_server, skill, tier))
}

fn spawn_parse_skill_card(
    parent: &mut ChildSpawnerCommands<'_>,
    font: &Handle<Font>,
    lang: crate::i18n::Language,
    card_w: f32,
    card_h: f32,
    face_image: Handle<Image>,
    title: &str,
    stats: SkillNumericStats,
) {
    parent
        .spawn((Node {
            width: Val::Px(card_w),
            height: Val::Px(card_h + 28.0),
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Center,
            ..default()
        },))
        .with_children(|card| {
            // Title label above card
            card.spawn((
                Text::new(title.to_string()),
                TextFont {
                    font: font.clone().into(),
                    font_size: FontSize::from(14.0),
                    ..default()
                },
                TextColor(skin::text_accent()),
                Node {
                    margin: UiRect::bottom(Val::Px(6.0)),
                    ..default()
                },
            ));
            // Card image with overlaid values
            card.spawn((
                Button,
                Node {
                    width: Val::Px(card_w),
                    height: Val::Px(card_h),
                    ..default()
                },
                BackgroundColor(Color::NONE),
                BorderColor::all(Color::NONE),
                UiTransform::IDENTITY,
                ZIndex(1),
            ))
            .with_children(|face| {
                face.spawn((
                    ImageNode::new(face_image),
                    Node {
                        position_type: PositionType::Absolute,
                        left: Val::Px(0.0),
                        top: Val::Px(0.0),
                        width: Val::Percent(100.0),
                        height: Val::Percent(100.0),
                        ..default()
                    },
                    ZIndex(1),
                ));
                // Damage value
                face.spawn((
                    Text::new(format_card_damage_value(stats.damage)),
                    TextLayout::justify(Justify::Center),
                    TextFont {
                        font: font.clone().into(),
                        font_size: FontSize::from(15.0),
                        ..default()
                    },
                    TextColor(Color::srgb(0.70, 0.08, 0.10)),
                    Node {
                        position_type: PositionType::Absolute,
                        left: Val::Px(card_w * SKILL_FACE_DAMAGE_CENTER_X - 48.0 * 0.5),
                        top: Val::Px(card_h * SKILL_PARSE_FACE_VALUE_CENTER_Y - 20.0 * 0.5),
                        width: Val::Px(48.0),
                        height: Val::Px(20.0),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        ..default()
                    },
                    ZIndex(3),
                ));
                // Cooldown value
                face.spawn((
                    Text::new(format_card_cooldown_value(stats.cooldown, lang)),
                    TextLayout::justify(Justify::Center),
                    TextFont {
                        font: font.clone().into(),
                        font_size: FontSize::from(12.0),
                        ..default()
                    },
                    TextColor(Color::srgb(0.04, 0.04, 0.04)),
                    Node {
                        position_type: PositionType::Absolute,
                        left: Val::Px(card_w * SKILL_FACE_COOLDOWN_CENTER_X - 66.0 * 0.5),
                        top: Val::Px(card_h * SKILL_PARSE_FACE_VALUE_CENTER_Y - 18.0 * 0.5),
                        width: Val::Px(66.0),
                        height: Val::Px(18.0),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        ..default()
                    },
                    ZIndex(3),
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
    parseable_q: Query<(
        &Transform,
        Option<&EnemyHitbox>,
        &ParseableSkill,
        Option<&Health>,
    )>,
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
    mut request_open_bag: ResMut<SkillBagOpenRequest>,
    carried: Res<CarriedSkills>,
    player_memory_q: Query<&PlayerMemory, With<Player>>,
    pool: Res<SkillPool>,
    skill_runtime: Res<SkillRuntimeStats>,
    asset_server: Res<AssetServer>,
    settings: Res<GameSettings>,
    popup_root_q: Query<Entity, With<SkillParsePopupRoot>>,
    stale_ui_q: Query<
        Entity,
        Or<(
            With<SkillBagUiRoot>,
            With<SkillBagDetailRoot>,
            With<EquipmentUiRoot>,
        )>,
    >,
    current_state: Res<State<GameState>>,
    mut next_state: ResMut<NextState<GameState>>,
    mut commands: Commands,
) {
    if !popup_root_q.is_empty() || popup_state.offer.is_some() {
        return;
    }

    let Some(skill) = pending_pick.0.take() else {
        return;
    };

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
    let def = pool.def(skill);
    let candidate = roll_stolen_skill_stats(SkillNumericStats::new(def.damage, def.cooldown));
    let offer = SkillParseOffer {
        skill,
        candidate,
        current: current_stats,
    };
    popup_state.offer = Some(offer);
    popup_state.wait_mouse_release = true;
    request_open_bag.0 = false;

    for root in &stale_ui_q {
        commands.entity(root).try_despawn();
    }

    spawn_skill_parse_popup_ui(
        &mut commands,
        &asset_server,
        &pool,
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

        let marker_sprite = Sprite {
            color: skin::text_accent(),
            custom_size: Some(Vec2::new(10.0, 10.0)),
            ..Default::default()
        };

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
    cards_q: Query<(&SkillCard, &CardAnim)>,
    cooldowns: Res<SkillCooldowns>,
    pool: Res<SkillPool>,
    runtime_stats: Res<SkillRuntimeStats>,
    mut text_q: ParamSet<(
        Query<(&SkillNameText, &mut Text)>,
        Query<(&SkillCooldownText, &mut Text)>,
        Query<(&SkillCardFallbackText, &mut TextColor)>,
        Query<(&SkillCardDamageValueText, &mut Text, &mut TextColor)>,
        Query<(&SkillCardCooldownValueText, &mut Text, &mut TextColor)>,
    )>,
    settings: Res<GameSettings>,
) {
    let lang = settings.language;

    for (marker, mut name_text) in &mut text_q.p0() {
        let skill = cards_q
            .iter()
            .find(|(c, _)| c.slot_index == marker.slot_index)
            .and_then(|(c, _)| c.skill);

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
            .find(|(c, _)| c.slot_index == marker.slot_index)
            .and_then(|(c, _)| c.skill);

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

    for (marker, mut color) in &mut text_q.p2() {
        let slot = cards_q
            .iter()
            .find(|(c, _)| c.slot_index == marker.slot_index);
        let empty = slot.map(|(c, _)| c.skill.is_none()).unwrap_or(true);
        let face_down = slot
            .map(|(_, anim)| {
                matches!(anim.phase, CardPhase::Drawing)
                    && anim.from_deck
                    && anim.timer.fraction() < 0.5
            })
            .unwrap_or(false);
        let faced = slot
            .and_then(|(c, _)| c.skill)
            .map(skill_has_card_face)
            .unwrap_or(false);
        color.0 = marker.color.with_alpha(if faced || empty || face_down {
            0.0
        } else {
            1.0
        });
    }

    for (marker, mut value_text, mut color) in &mut text_q.p3() {
        let slot = cards_q
            .iter()
            .find(|(c, _)| c.slot_index == marker.slot_index);
        let face_down = slot
            .map(|(_, anim)| {
                matches!(anim.phase, CardPhase::Drawing)
                    && anim.from_deck
                    && anim.timer.fraction() < 0.5
            })
            .unwrap_or(false);
        let skill = slot.and_then(|(c, _)| c.skill);
        if let Some(skill) = skill.filter(|skill| skill_has_card_face(*skill)) {
            let stats = effective_skill_stats(skill, &pool, &runtime_stats);
            *value_text = Text::new(format_card_damage_value(stats.damage));
            color.0 = if face_down {
                Color::NONE
            } else {
                Color::srgb(0.70, 0.08, 0.10)
            };
        } else {
            *value_text = Text::new("");
            color.0 = Color::NONE;
        }
    }

    for (marker, mut value_text, mut color) in &mut text_q.p4() {
        let slot = cards_q
            .iter()
            .find(|(c, _)| c.slot_index == marker.slot_index);
        let face_down = slot
            .map(|(_, anim)| {
                matches!(anim.phase, CardPhase::Drawing)
                    && anim.from_deck
                    && anim.timer.fraction() < 0.5
            })
            .unwrap_or(false);
        let skill = slot.and_then(|(c, _)| c.skill);
        if let Some(skill) = skill.filter(|skill| skill_has_card_face(*skill)) {
            let stats = effective_skill_stats(skill, &pool, &runtime_stats);
            *value_text = Text::new(format_card_cooldown_value(stats.cooldown, lang));
            color.0 = if face_down {
                Color::NONE
            } else {
                Color::srgb(0.04, 0.04, 0.04)
            };
        } else {
            *value_text = Text::new("");
            color.0 = Color::NONE;
        }
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
        for root in &equipment_ui_q {
            commands.entity(root).try_despawn();
        }
        if matches!(current_state.get(), GameState::Paused) {
            suppress_pause_menu_once.0 = false;
            next_state.set(GameState::InGame);
        }
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
    detail_q: Query<Entity, With<SkillBagDetailRoot>>,
    current_state: Res<State<GameState>>,
    mut next_state: ResMut<NextState<GameState>>,
    mut suppress_pause_menu_once: ResMut<SuppressPauseMenuOnce>,
) {
    if !keyboard.just_pressed(KeyCode::Escape) {
        return;
    }

    if let Ok(detail) = detail_q.single() {
        commands.entity(detail).try_despawn();
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

    if popup_q.is_empty() {
        return;
    }

    for root in &popup_q {
        commands.entity(root).try_despawn();
    }
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
    if popup_q.is_empty() {
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
                        request_open_bag.0 = false;
                        for root in &popup_q {
                            commands.entity(root).try_despawn();
                        }
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

                        if !changed {
                            continue;
                        }

                        dirty.0 = true;
                        popup_state.offer = None;
                        popup_state.wait_mouse_release = false;
                        for root in &popup_q {
                            commands.entity(root).try_despawn();
                        }
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

    if !popup_q.is_empty() {
        return;
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
        (
            Changed<Interaction>,
            With<Button>,
            With<SkillBagCloseButton>,
        ),
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

fn handle_skill_bag_card_clicks(
    mut commands: Commands,
    interactions: Query<(&Interaction, &SkillBagCard), (Changed<Interaction>, With<Button>)>,
    root_q: Query<Entity, With<SkillBagUiRoot>>,
    detail_q: Query<Entity, With<SkillBagDetailRoot>>,
    asset_server: Res<AssetServer>,
    pool: Res<SkillPool>,
    runtime_stats: Res<SkillRuntimeStats>,
    settings: Res<GameSettings>,
) {
    if !detail_q.is_empty() {
        return;
    }

    let Ok(root) = root_q.single() else {
        return;
    };

    for (interaction, card) in &interactions {
        if !matches!(interaction, Interaction::Pressed) {
            continue;
        }

        let skill = card.skill;
        let stats = effective_skill_stats(skill, &pool, &runtime_stats);
        let face_tier = skill_card_face_tier(skill, &pool, &runtime_stats);
        spawn_skill_bag_detail_overlay(
            &mut commands,
            root,
            &asset_server,
            skill,
            stats,
            face_tier,
            settings.language,
        );
        break;
    }
}

fn handle_skill_bag_detail_close_button(
    mut commands: Commands,
    mut interactions: Query<
        (
            &Interaction,
            &mut BackgroundColor,
            Option<&SkillBagDetailBackdrop>,
        ),
        (
            Changed<Interaction>,
            With<Button>,
            Or<(
                With<SkillBagDetailCloseButton>,
                With<SkillBagDetailBackdrop>,
            )>,
        ),
    >,
    detail_q: Query<Entity, With<SkillBagDetailRoot>>,
) {
    for (interaction, mut bg, backdrop) in &mut interactions {
        match *interaction {
            Interaction::Pressed => {
                if backdrop.is_none() {
                    bg.0 = skin::button_pressed();
                }
                if let Ok(detail) = detail_q.single() {
                    commands.entity(detail).try_despawn();
                }
            }
            Interaction::Hovered => {
                if backdrop.is_none() {
                    bg.0 = skin::button_hover();
                }
            }
            Interaction::None => {
                if backdrop.is_none() {
                    bg.0 = skin::button_danger();
                }
            }
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

fn animate_skill_bag_cards(
    time: Res<Time>,
    mut cards: Query<(
        &SkillBagCard,
        &mut SkillBagCardAnim,
        &Interaction,
        &mut UiTransform,
        &mut ZIndex,
        &mut BackgroundColor,
        &mut BorderColor,
    )>,
) {
    let dt = time.delta_secs();
    let now = time.elapsed_secs();
    let hover_smooth = 1.0 - (-18.0 * dt).exp();

    for (card, mut anim, interaction, mut transform, mut z, mut fill, mut border) in &mut cards {
        anim.age += dt;

        let hovered = matches!(interaction, Interaction::Hovered | Interaction::Pressed);
        let pressed = matches!(interaction, Interaction::Pressed);
        let target_hover = if hovered { 1.0 } else { 0.0 };
        anim.hover += (target_hover - anim.hover) * hover_smooth;

        let deal_delay = card.slot_index as f32 * SKILL_BAG_CARD_DEAL_STAGGER;
        let deal_t = ((anim.age - deal_delay) / SKILL_BAG_CARD_DEAL_TIME).clamp(0.0, 1.0);
        let deal = crate::ui::anim::Ease::OutBack.apply(deal_t);

        let col = (card.slot_index % SKILL_BAG_GRID_COLUMNS) as f32;
        let row = (card.slot_index / SKILL_BAG_GRID_COLUMNS) as f32;
        let rel_col = col - (SKILL_BAG_GRID_COLUMNS as f32 - 1.0) * 0.5;
        let idle_wave = (now * 2.25 + anim.seed).sin();
        let _pulse = (now * 3.2 + anim.seed * 0.7).sin() * 0.5 + 0.5;

        let deal_x = rel_col * 18.0;
        let deal_y = 52.0 + row * 26.0;
        let hover_x = idle_wave * 1.8 * anim.hover;
        let hover_y = -SKILL_BAG_CARD_HOVER_LIFT * anim.hover + if pressed { 5.0 } else { 0.0 };
        transform.translation = Val2::px(
            deal_x * (1.0 - deal) + hover_x,
            deal_y * (1.0 - deal) + hover_y,
        );

        let base_rot = rel_col * 0.022 + (row - 0.5) * 0.01;
        let hover_rot = rel_col * 0.055 + idle_wave * 0.018;
        let idle_rot = idle_wave * 0.004;
        let rotation = base_rot * (1.0 - anim.hover) + hover_rot * anim.hover + idle_rot;
        transform.rotation = Rot2::radians(rotation * deal);

        let deal_scale = 0.72 + 0.28 * deal;
        let hover_scale = 1.0 + 0.085 * anim.hover;
        let press_scale = if pressed { 0.965 } else { 1.0 };
        transform.scale = Vec2::splat(deal_scale * hover_scale * press_scale);

        fill.0 = Color::NONE;
        *border = BorderColor::all(Color::NONE);

        *z = ZIndex(if hovered {
            1000 + card.slot_index as i32
        } else {
            20 + card.slot_index as i32
        });
    }
}

fn spawn_skill_bag_detail_overlay(
    commands: &mut Commands,
    root: Entity,
    asset_server: &AssetServer,
    skill: SkillId,
    stats: SkillNumericStats,
    face_tier: SkillCardFaceTier,
    lang: crate::i18n::Language,
) {
    let font = skin::ui_font(asset_server, lang);
    let face_image = face_image_for_skill(asset_server, skill, face_tier);
    let img_w = 318.0;
    let img_h = 390.0;

    let mut cmds = commands.entity(root);
    cmds.with_children(|parent| {
        parent
            .spawn((
                SkillBagDetailRoot,
                SkillBagDetailBackdrop,
                Button,
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
                BackgroundColor(skin::overlay().with_alpha(0.82)),
                UiTransform::IDENTITY,
                ZIndex(3000),
            ))
            .with_children(|overlay| {
                overlay
                    .spawn((
                        Button,
                        SkillBagDetailCardAnim { age: 0.0 },
                        Node {
                            width: Val::Px(img_w),
                            height: Val::Px(img_h),
                            position_type: PositionType::Relative,
                            ..default()
                        },
                        BackgroundColor(Color::NONE),
                        BorderColor::all(Color::NONE),
                        UiTransform::IDENTITY,
                        crate::ui::anim::UiTween::pop_in(0.32, 0.94),
                        ZIndex(3001),
                    ))
                    .with_children(|card| {
                        // Face image
                        card.spawn((
                            ImageNode::new(face_image),
                            Node {
                                position_type: PositionType::Absolute,
                                left: Val::Px(0.0),
                                top: Val::Px(0.0),
                                width: Val::Percent(100.0),
                                height: Val::Percent(100.0),
                                ..default()
                            },
                            ZIndex(1),
                        ));
                        // Damage value
                        card.spawn((
                            Text::new(format_card_damage_value(stats.damage)),
                            TextLayout::justify(Justify::Center),
                            TextFont {
                                font: font.clone().into(),
                                font_size: FontSize::from(28.0),
                                ..default()
                            },
                            TextColor(Color::srgb(0.70, 0.08, 0.10)),
                            Node {
                                position_type: PositionType::Absolute,
                                left: Val::Px(img_w * SKILL_FACE_DAMAGE_CENTER_X - 70.0 * 0.5),
                                top: Val::Px(img_h * SKILL_FACE_VALUE_CENTER_Y - 36.0 * 0.5),
                                width: Val::Px(70.0),
                                height: Val::Px(36.0),
                                justify_content: JustifyContent::Center,
                                align_items: AlignItems::Center,
                                ..default()
                            },
                            ZIndex(3),
                        ));
                        // Cooldown value
                        card.spawn((
                            Text::new(format_card_cooldown_value(stats.cooldown, lang)),
                            TextLayout::justify(Justify::Center),
                            TextFont {
                                font: font.clone().into(),
                                font_size: FontSize::from(22.0),
                                ..default()
                            },
                            TextColor(Color::srgb(0.04, 0.04, 0.04)),
                            Node {
                                position_type: PositionType::Absolute,
                                left: Val::Px(img_w * SKILL_FACE_COOLDOWN_CENTER_X - 80.0 * 0.5),
                                top: Val::Px(img_h * SKILL_FACE_VALUE_CENTER_Y - 30.0 * 0.5),
                                width: Val::Px(80.0),
                                height: Val::Px(30.0),
                                justify_content: JustifyContent::Center,
                                align_items: AlignItems::Center,
                                ..default()
                            },
                            ZIndex(3),
                        ));
                        // Close button
                        card.spawn((
                            Button,
                            SkillBagDetailCloseButton,
                            Node {
                                position_type: PositionType::Absolute,
                                right: Val::Px(-12.0),
                                top: Val::Px(-12.0),
                                width: Val::Px(36.0),
                                height: Val::Px(36.0),
                                justify_content: JustifyContent::Center,
                                align_items: AlignItems::Center,
                                border: UiRect::all(Val::Px(1.5)),
                                border_radius: BorderRadius::all(Val::Px(skin::BUTTON_RADIUS)),
                                ..default()
                            },
                            BackgroundColor(skin::button_danger()),
                            BorderColor::all(skin::border_soft()),
                            skin::shadow_card(),
                            UiTransform::IDENTITY,
                            ZIndex(5),
                        ))
                        .with_children(|close| {
                            close.spawn((
                                Text::new("X"),
                                TextFont {
                                    font: font.clone().into(),
                                    font_size: FontSize::from(18.0),
                                    ..default()
                                },
                                TextColor(skin::text_primary()),
                            ));
                        });
                    });
            });
    });
}

fn animate_skill_bag_detail_card(
    time: Res<Time>,
    mut cards: Query<(&mut SkillBagDetailCardAnim, &mut UiTransform)>,
) {
    let dt = time.delta_secs();
    let now = time.elapsed_secs();

    for (mut anim, mut transform) in &mut cards {
        anim.age += dt;
        let intro = crate::ui::anim::Ease::OutBack.apply((anim.age / 0.34).clamp(0.0, 1.0));
        let breathe = (now * 2.0).sin() * 0.5 + 0.5;
        let float = (now * 1.45).sin();

        transform.translation = Val2::px(0.0, (1.0 - intro) * 52.0 + float * 3.0 * intro);
        transform.scale = Vec2::splat((0.78 + 0.22 * intro) * (1.0 + 0.012 * breathe));
        transform.rotation = Rot2::radians(float * 0.01 * intro);
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
    let font = skin::ui_font(asset_server, lang);
    let carried_list = carried_skill_types_for_bag(carried);
    let carried_count = carried_non_dash_count(carried);
    let total_cards = carried_list.len().max(1);
    let total_pages = total_cards.div_ceil(SKILL_BAG_PAGE_SIZE);
    page_state.current_page = page_state.current_page.min(total_pages.saturating_sub(1));
    let current_page = page_state.current_page;
    let start = current_page * SKILL_BAG_PAGE_SIZE;

    let card_w = 132.0;
    let card_h = 194.0;

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
                    row_gap: Val::Px(12.0),
                    padding: UiRect::all(Val::Px(18.0)),
                    border: UiRect::all(Val::Px(1.5)),
                    border_radius: BorderRadius::all(Val::Px(skin::PANEL_RADIUS)),
                    ..default()
                },
                BackgroundColor(skin::panel_tint()),
                skin::vgradient(skin::panel_grad_top(), skin::panel_grad_bottom()),
                BorderColor::all(skin::border_soft()),
                UiTransform::IDENTITY,
                crate::ui::anim::UiTween::pop_in(0.28, 0.95),
            ))
            .with_children(|panel| {
                panel
                    .spawn((Node {
                        width: Val::Percent(100.0),
                        justify_content: JustifyContent::SpaceBetween,
                        align_items: AlignItems::FlexStart,
                        ..default()
                    },))
                    .with_children(|header| {
                        header
                            .spawn((Node {
                                flex_direction: FlexDirection::Column,
                                row_gap: Val::Px(6.0),
                                align_items: AlignItems::FlexStart,
                                ..default()
                            },))
                            .with_children(|left| {
                                left.spawn((
                                    Text::new(L10n::skill_backpack_title(lang)),
                                    TextFont {
                                        font: font.clone().into(),
                                        font_size: FontSize::from(24.0),
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
                                        font: font.clone().into(),
                                        font_size: FontSize::from(16.0),
                                        ..default()
                                    },
                                    TextColor(skin::text_muted()),
                                ));
                            });

                        header
                            .spawn((Node {
                                width: Val::Px(280.0),
                                flex_direction: FlexDirection::Column,
                                row_gap: Val::Px(6.0),
                                align_items: AlignItems::FlexEnd,
                                ..default()
                            },))
                            .with_children(|right| {
                                right.spawn((
                                    Text::new(L10n::skill_backpack_page(
                                        lang,
                                        current_page + 1,
                                        total_pages,
                                    )),
                                    TextFont {
                                        font: font.clone().into(),
                                        font_size: FontSize::from(15.0),
                                        ..default()
                                    },
                                    TextColor(skin::text_muted()),
                                ));

                                right
                                    .spawn((Node {
                                        flex_direction: FlexDirection::Row,
                                        column_gap: Val::Px(8.0),
                                        align_items: AlignItems::Center,
                                        justify_content: JustifyContent::FlexEnd,
                                        ..default()
                                    },))
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

                panel
                    .spawn((
                        Node {
                            width: Val::Percent(100.0),
                            flex_grow: 1.0,
                            display: Display::Grid,
                            grid_auto_flow: GridAutoFlow::Column,
                            justify_content: JustifyContent::Center,
                            align_content: AlignContent::Start,
                            grid_template_columns: RepeatedGridTrack::px(5, card_w),
                            grid_template_rows: RepeatedGridTrack::px(2, card_h),
                            row_gap: Val::Px(10.0),
                            column_gap: Val::Px(14.0),
                            padding: UiRect::all(Val::Px(10.0)),
                            border_radius: BorderRadius::all(Val::Px(12.0)),
                            overflow: Overflow::clip(),
                            ..default()
                        },
                        BackgroundColor(skin::inset_tint()),
                    ))
                    .with_children(|grid| {
                        for i in 0..SKILL_BAG_PAGE_SIZE {
                            let maybe = carried_list.get(start + i).copied();
                            match maybe {
                                Some(skill) => {
                                    let _def = pool.def(skill);
                                    let stats = effective_skill_stats(skill, pool, runtime_stats);
                                    let face_tier =
                                        skill_card_face_tier(skill, pool, runtime_stats);
                                    let face_image = optional_face_image_for_skill(
                                        asset_server,
                                        skill,
                                        face_tier,
                                    );
                                    grid.spawn((
                                        Button,
                                        SkillBagCard {
                                            slot_index: i,
                                            skill,
                                        },
                                        SkillBagCardAnim::new(i),
                                        Node {
                                            width: Val::Px(card_w),
                                            height: Val::Px(card_h),
                                            padding: UiRect::all(Val::Px(3.0)),
                                            ..default()
                                        },
                                        BackgroundColor(Color::NONE),
                                        BorderColor::all(Color::NONE),
                                        UiTransform::IDENTITY,
                                        ZIndex(i as i32),
                                    ))
                                    .with_children(|card| {
                                        if let Some(ref img) = face_image {
                                            card.spawn((
                                                ImageNode::new(img.clone()),
                                                Node {
                                                    position_type: PositionType::Absolute,
                                                    left: Val::Px(0.0),
                                                    top: Val::Px(0.0),
                                                    width: Val::Percent(100.0),
                                                    height: Val::Percent(100.0),
                                                    ..default()
                                                },
                                                ZIndex(1),
                                            ));
                                            // Damage value
                                            card.spawn((
                                                Text::new(format_card_damage_value(stats.damage)),
                                                TextLayout::justify(Justify::Center),
                                                TextFont {
                                                    font: font.clone().into(),
                                                    font_size: FontSize::from(15.0),
                                                    ..default()
                                                },
                                                TextColor(Color::srgb(0.70, 0.08, 0.10)),
                                                Node {
                                                    position_type: PositionType::Absolute,
                                                    left: Val::Px(
                                                        card_w * SKILL_FACE_DAMAGE_CENTER_X
                                                            - 38.0 * 0.5,
                                                    ),
                                                    top: Val::Px(card_h * 0.66 - 18.0 * 0.5),
                                                    width: Val::Px(38.0),
                                                    height: Val::Px(18.0),
                                                    justify_content: JustifyContent::Center,
                                                    align_items: AlignItems::Center,
                                                    ..default()
                                                },
                                                ZIndex(3),
                                            ));
                                            // Cooldown value
                                            card.spawn((
                                                Text::new(format_card_cooldown_value(
                                                    stats.cooldown,
                                                    lang,
                                                )),
                                                TextLayout::justify(Justify::Center),
                                                TextFont {
                                                    font: font.clone().into(),
                                                    font_size: FontSize::from(12.0),
                                                    ..default()
                                                },
                                                TextColor(Color::srgb(0.04, 0.04, 0.04)),
                                                Node {
                                                    position_type: PositionType::Absolute,
                                                    left: Val::Px(
                                                        card_w * SKILL_FACE_COOLDOWN_CENTER_X
                                                            - 48.0 * 0.5,
                                                    ),
                                                    top: Val::Px(card_h * 0.66 - 16.0 * 0.5),
                                                    width: Val::Px(48.0),
                                                    height: Val::Px(16.0),
                                                    justify_content: JustifyContent::Center,
                                                    align_items: AlignItems::Center,
                                                    ..default()
                                                },
                                                ZIndex(3),
                                            ));
                                        } else {
                                            // Fallback: skill name only for
                                            // non-face skills
                                            card.spawn((
                                                Text::new(L10n::skill_name(lang, skill)),
                                                TextLayout::justify(Justify::Center),
                                                TextFont {
                                                    font: font.clone().into(),
                                                    font_size: FontSize::from(16.0),
                                                    ..default()
                                                },
                                                TextColor(skin::text_primary()),
                                                Node {
                                                    width: Val::Percent(100.0),
                                                    height: Val::Percent(100.0),
                                                    justify_content: JustifyContent::Center,
                                                    align_items: AlignItems::Center,
                                                    ..default()
                                                },
                                                ZIndex(2),
                                            ));
                                        }
                                    });
                                }
                                None => {
                                    grid.spawn((Node {
                                        width: Val::Px(card_w),
                                        height: Val::Px(card_h),
                                        ..default()
                                    },));
                                }
                            }
                        }
                    });
            });
        });
}

fn spawn_skill_bag_small_button(
    parent: &mut ChildSpawnerCommands<'_>,
    _asset_server: &AssetServer,
    font: &Handle<Font>,
    label: &str,
    page_btn: SkillBagPageButton,
) {
    parent
        .spawn((
            Button,
            page_btn,
            Node {
                width: Val::Px(92.0),
                height: Val::Px(34.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                border: UiRect::all(Val::Px(1.5)),
                border_radius: BorderRadius::all(Val::Px(skin::BUTTON_RADIUS)),
                ..default()
            },
            BackgroundColor(skin::button_idle()),
            BorderColor::all(skin::border_soft()),
            skin::shadow_card(),
            UiTransform::IDENTITY,
            crate::ui::anim::HoverMotion::default(),
        ))
        .with_children(|btn| {
            btn.spawn((
                Text::new(label),
                TextFont {
                    font: font.clone().into(),
                    font_size: FontSize::from(skin::FONT_CAPTION + 1.0),
                    ..default()
                },
                TextColor(skin::text_primary()),
            ));
        });
}

fn spawn_skill_bag_close_button(
    parent: &mut ChildSpawnerCommands<'_>,
    _asset_server: &AssetServer,
    font: &Handle<Font>,
    label: &str,
) {
    parent
        .spawn((
            Button,
            SkillBagCloseButton,
            Node {
                width: Val::Px(92.0),
                height: Val::Px(34.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                border: UiRect::all(Val::Px(1.5)),
                border_radius: BorderRadius::all(Val::Px(skin::BUTTON_RADIUS)),
                ..default()
            },
            BackgroundColor(skin::button_danger()),
            BorderColor::all(skin::border_soft()),
            skin::shadow_card(),
            UiTransform::IDENTITY,
            crate::ui::anim::HoverMotion::default(),
        ))
        .with_children(|btn| {
            btn.spawn((
                Text::new(label),
                TextFont {
                    font: font.clone().into(),
                    font_size: FontSize::from(skin::FONT_CAPTION + 1.0),
                    ..default()
                },
                TextColor(skin::text_primary()),
            ));
        });
}
