use bevy::prelude::*;
use bevy::ui::{UiRect, Val};
use bevy_ecs_ldtk::prelude::LevelSelection;

use crate::i18n::L10n;
use crate::input::MovementInput;
use crate::interaction::InteractEvent;
use crate::ldtk_collision::WallColliders;
use crate::movement::Player;
use crate::state::GameState;
use crate::ui::EscBlockingUi;
use crate::ui::skin;
use crate::ui::types::GameSettings;
use crate::utils::despawn_with_children;

const NPC_LEVEL_ID: &str = "Level_0";
const NPC_INTERACT_RANGE: f32 = 52.0;
const NPC_DIALOGUE_PAGE_COUNT: usize = 3;
const NPC_COLLIDER_HALF: Vec2 = Vec2::new(12.0, 14.0);
const NPC_SPAWN_MARGIN: f32 = 40.0;

pub struct DialoguePlugin;

#[derive(Component)]
struct DialogueNpc;

#[derive(Component, Clone, Copy)]
pub struct DialogueNpcCollider {
    pub half: Vec2,
}

#[derive(Component)]
struct DialogueUiRoot;

#[derive(Component)]
struct DialogueSpeakerText;

#[derive(Component)]
struct DialogueBodyText;

#[derive(Component)]
struct DialogueHintText;

#[derive(Resource)]
struct DialogueNpcTexture(Handle<Image>);

#[derive(Resource, Default)]
struct ActiveDialogue {
    page: usize,
}

impl Plugin for DialoguePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, load_npc_texture)
            .add_systems(
                Update,
                (
                    sync_dialogue_npc_presence,
                    update_npc_visual_state,
                    start_npc_dialogue,
                )
                    .run_if(in_state(GameState::InGame)),
            )
            .add_systems(
                OnEnter(GameState::Dialogue),
                (clear_movement_input, spawn_dialogue_ui),
            )
            .add_systems(
                Update,
                (advance_dialogue, sync_dialogue_ui).run_if(in_state(GameState::Dialogue)),
            )
            .add_systems(
                OnExit(GameState::Dialogue),
                (cleanup_dialogue_ui, clear_active_dialogue),
            )
            .add_systems(OnEnter(GameState::MainMenu), cleanup_dialogue_world);
    }
}

fn load_npc_texture(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.insert_resource(DialogueNpcTexture(asset_server.load("npc.png")));
}

fn sync_dialogue_npc_presence(
    mut commands: Commands,
    level_selection: Option<Res<LevelSelection>>,
    walls: Res<WallColliders>,
    texture: Res<DialogueNpcTexture>,
    npc_q: Query<Entity, With<DialogueNpc>>,
) {
    let should_exist = level_selection
        .as_deref()
        .and_then(current_level_name)
        .map(|level| level == NPC_LEVEL_ID)
        .unwrap_or(false);

    if !should_exist {
        for entity in &npc_q {
            commands.entity(entity).despawn();
        }
        return;
    }

    if !npc_q.is_empty() {
        return;
    }

    if walls.bounds.is_none() {
        return;
    }

    let mut sprite = Sprite::from_image(texture.0.clone());
    sprite.custom_size = Some(Vec2::new(34.0, 42.0));

    commands.spawn((
        DialogueNpc,
        DialogueNpcCollider {
            half: NPC_COLLIDER_HALF,
        },
        sprite,
        Transform::from_translation(pick_npc_spawn_position(&walls).extend(10.0)),
    ));
}

fn update_npc_visual_state(
    player_q: Query<&Transform, With<Player>>,
    mut npc_q: Query<(&Transform, &mut Sprite), With<DialogueNpc>>,
) {
    let Ok(player_tf) = player_q.single() else {
        return;
    };
    let player_pos = player_tf.translation.truncate();

    for (npc_tf, mut sprite) in &mut npc_q {
        let distance = npc_tf.translation.truncate().distance(player_pos);
        let nearby = distance <= NPC_INTERACT_RANGE;

        sprite.color = if nearby {
            Color::srgb(1.0, 0.96, 0.82)
        } else {
            Color::WHITE
        };
    }
}

fn start_npc_dialogue(
    mut commands: Commands,
    mut interact_events: MessageReader<InteractEvent>,
    mut next_state: ResMut<NextState<GameState>>,
    player_q: Query<&Transform, With<Player>>,
    npc_q: Query<&Transform, With<DialogueNpc>>,
) {
    if interact_events.is_empty() {
        return;
    }

    let triggered = interact_events.read().next().is_some();
    if !triggered {
        return;
    }

    let Ok(player_tf) = player_q.single() else {
        return;
    };
    let npc_nearby = npc_q
        .iter()
        .filter(|npc_tf| {
            npc_tf
                .translation
                .truncate()
                .distance(player_tf.translation.truncate())
                <= NPC_INTERACT_RANGE
        })
        .min_by(|a, b| {
            a.translation
                .truncate()
                .distance_squared(player_tf.translation.truncate())
                .partial_cmp(
                    &b.translation
                        .truncate()
                        .distance_squared(player_tf.translation.truncate()),
                )
                .unwrap_or(std::cmp::Ordering::Equal)
        })
        .is_some();

    if !npc_nearby {
        return;
    }

    commands.insert_resource(ActiveDialogue::default());
    next_state.set(GameState::Dialogue);
}

fn clear_movement_input(mut movement: ResMut<MovementInput>) {
    movement.0 = Vec2::ZERO;
}

fn spawn_dialogue_ui(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    settings: Res<GameSettings>,
    active: Res<ActiveDialogue>,
) {
    let font: Handle<Font> = asset_server.load("fonts/YuFanLixing.otf");
    let lang = settings.language;
    let npc_image: Handle<Image> = asset_server.load("npc.png");

    commands
        .spawn((
            DialogueUiRoot,
            EscBlockingUi,
            GlobalZIndex(320),
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                position_type: PositionType::Absolute,
                left: Val::Px(0.0),
                top: Val::Px(0.0),
                justify_content: JustifyContent::FlexEnd,
                align_items: AlignItems::Center,
                padding: UiRect::new(Val::Px(18.0), Val::Px(18.0), Val::Px(18.0), Val::Px(0.0)),
                ..default()
            },
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.12)),
        ))
        .with_children(|root| {
            root.spawn((
                Node {
                    width: Val::Percent(88.0),
                    max_width: Val::Px(920.0),
                    min_height: Val::Px(210.0),
                    margin: UiRect::bottom(Val::Px(20.0)),
                    padding: UiRect::all(Val::Px(16.0)),
                    column_gap: Val::Px(16.0),
                    align_items: AlignItems::Stretch,
                    justify_content: JustifyContent::FlexStart,
                    ..default()
                },
                BackgroundColor(skin::panel_tint()),
                ImageNode::new(skin::panel(&asset_server)),
            ))
            .with_children(|panel| {
                panel
                    .spawn((
                        Node {
                            width: Val::Px(180.0),
                            min_height: Val::Px(180.0),
                            justify_content: JustifyContent::Center,
                            align_items: AlignItems::Center,
                            padding: UiRect::all(Val::Px(12.0)),
                            ..default()
                        },
                        BackgroundColor(skin::inset_tint()),
                        ImageNode::new(skin::panel(&asset_server)),
                    ))
                    .with_children(|portrait| {
                        portrait.spawn((
                            ImageNode::new(npc_image),
                            Node {
                                width: Val::Px(112.0),
                                height: Val::Px(140.0),
                                ..default()
                            },
                        ));
                    });

                panel
                    .spawn((Node {
                        flex_grow: 1.0,
                        min_height: Val::Px(180.0),
                        flex_direction: FlexDirection::Column,
                        justify_content: JustifyContent::SpaceBetween,
                        row_gap: Val::Px(14.0),
                        ..default()
                    },))
                    .with_children(|content| {
                        content.spawn((
                            DialogueSpeakerText,
                            Text::new(L10n::dialogue_npc_name(lang)),
                            TextFont {
                                font: font.clone(),
                                font_size: 30.0,
                                ..default()
                            },
                            TextColor(skin::text_accent()),
                        ));

                        content.spawn((
                            DialogueBodyText,
                            Text::new(L10n::dialogue_page(lang, active.page)),
                            TextFont {
                                font: font.clone(),
                                font_size: 22.0,
                                ..default()
                            },
                            TextColor(skin::text_primary()),
                            Node {
                                width: Val::Percent(100.0),
                                ..default()
                            },
                        ));

                        content
                            .spawn((Node {
                                width: Val::Percent(100.0),
                                justify_content: JustifyContent::FlexEnd,
                                ..default()
                            },))
                            .with_children(|footer| {
                                footer.spawn((
                                    DialogueHintText,
                                    Text::new(dialogue_hint_text(lang, active.page)),
                                    TextFont {
                                        font: font.clone(),
                                        font_size: 18.0,
                                        ..default()
                                    },
                                    TextColor(skin::text_muted()),
                                ));
                            });
                    });
            });
        });
}

fn advance_dialogue(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut active: ResMut<ActiveDialogue>,
    mut next_state: ResMut<NextState<GameState>>,
) {
    let should_advance = keyboard.just_pressed(KeyCode::KeyE)
        || keyboard.just_pressed(KeyCode::Enter)
        || keyboard.just_pressed(KeyCode::Space);
    let should_close = keyboard.just_pressed(KeyCode::Escape);

    if should_close {
        next_state.set(GameState::InGame);
        return;
    }

    if !should_advance {
        return;
    }

    if active.page + 1 >= NPC_DIALOGUE_PAGE_COUNT {
        next_state.set(GameState::InGame);
    } else {
        active.page += 1;
    }
}

fn sync_dialogue_ui(
    active: Res<ActiveDialogue>,
    settings: Res<GameSettings>,
    mut text_q: Query<(
        &mut Text,
        AnyOf<(&DialogueSpeakerText, &DialogueBodyText, &DialogueHintText)>,
    )>,
) {
    if !(active.is_changed() || settings.is_changed()) {
        return;
    }

    let lang = settings.language;

    for (mut text, (is_speaker, is_body, is_hint)) in &mut text_q {
        if is_speaker.is_some() {
            text.0 = L10n::dialogue_npc_name(lang).to_string();
        } else if is_body.is_some() {
            text.0 = L10n::dialogue_page(lang, active.page).to_string();
        } else if is_hint.is_some() {
            text.0 = dialogue_hint_text(lang, active.page).to_string();
        }
    }
}

fn cleanup_dialogue_ui(
    mut commands: Commands,
    root_q: Query<Entity, With<DialogueUiRoot>>,
    children_q: Query<&Children>,
) {
    if let Ok(root) = root_q.single() {
        despawn_with_children(&mut commands, &children_q, root);
    }
}

fn clear_active_dialogue(mut commands: Commands) {
    commands.remove_resource::<ActiveDialogue>();
}

fn cleanup_dialogue_world(
    mut commands: Commands,
    npc_q: Query<Entity, With<DialogueNpc>>,
    ui_q: Query<Entity, With<DialogueUiRoot>>,
    children_q: Query<&Children>,
) {
    for entity in &npc_q {
        commands.entity(entity).despawn();
    }

    if let Ok(root) = ui_q.single() {
        despawn_with_children(&mut commands, &children_q, root);
    }

    commands.remove_resource::<ActiveDialogue>();
}

fn dialogue_hint_text(lang: crate::i18n::Language, page: usize) -> &'static str {
    if page + 1 >= NPC_DIALOGUE_PAGE_COUNT {
        L10n::dialogue_close_hint(lang)
    } else {
        L10n::dialogue_advance_hint(lang)
    }
}

fn current_level_name(selection: &LevelSelection) -> Option<&str> {
    match selection {
        LevelSelection::Identifier(name) => Some(name.as_str()),
        LevelSelection::Indices(indices) => match (indices.world, indices.level) {
            (None, 0) => Some("Level_0"),
            (None, 1) => Some("Level_1"),
            _ => None,
        },
        _ => None,
    }
}

fn pick_npc_spawn_position(walls: &WallColliders) -> Vec2 {
    let desired = walls
        .bounds
        .map(|(min, max)| {
            let raw_inner_min = min + Vec2::splat(NPC_SPAWN_MARGIN);
            let raw_inner_max = max - Vec2::splat(NPC_SPAWN_MARGIN);
            let inner_min = Vec2::new(
                raw_inner_min.x.min(raw_inner_max.x),
                raw_inner_min.y.min(raw_inner_max.y),
            );
            let inner_max = Vec2::new(
                raw_inner_min.x.max(raw_inner_max.x),
                raw_inner_min.y.max(raw_inner_max.y),
            );
            let target = (min + max) * 0.5 + Vec2::new(-96.0, -24.0);
            Vec2::new(
                target.x.clamp(inner_min.x, inner_max.x),
                target.y.clamp(inner_min.y, inner_max.y),
            )
        })
        .unwrap_or(Vec2::new(192.0, 192.0));

    let bounded_walkable = walls.walkables.iter().copied().filter(|cell| {
        walls
            .bounds
            .map(|(min, max)| {
                let raw_inner_min = min + Vec2::splat(NPC_SPAWN_MARGIN);
                let raw_inner_max = max - Vec2::splat(NPC_SPAWN_MARGIN);
                let inner_min = Vec2::new(
                    raw_inner_min.x.min(raw_inner_max.x),
                    raw_inner_min.y.min(raw_inner_max.y),
                );
                let inner_max = Vec2::new(
                    raw_inner_min.x.max(raw_inner_max.x),
                    raw_inner_min.y.max(raw_inner_max.y),
                );

                cell.x >= inner_min.x
                    && cell.x <= inner_max.x
                    && cell.y >= inner_min.y
                    && cell.y <= inner_max.y
            })
            .unwrap_or(true)
    });

    bounded_walkable
        .min_by(|a, b| {
            a.distance_squared(desired)
                .partial_cmp(&b.distance_squared(desired))
                .unwrap_or(std::cmp::Ordering::Equal)
        })
        .or_else(|| {
            walls.walkables.iter().copied().min_by(|a, b| {
                a.distance_squared(desired)
                    .partial_cmp(&b.distance_squared(desired))
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
        })
        .unwrap_or(desired)
}
