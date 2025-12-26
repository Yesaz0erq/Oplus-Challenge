use bevy::prelude::*;

use crate::combat_core::{skill_slash, spawn_slash_vfx, CombatSet, VfxPool};
use crate::enemy::Enemy;
use crate::health::Health;
use crate::movement::{Player, PlayerAnimation, PlayerDash};
use crate::skills_pool::{SkillId, SkillPool};
use crate::state::GameState;

const MAX_SKILL_CARDS: usize = 3;
const SKILL_CARD_SIZE: f32 = 64.0;

#[derive(Component)]
struct SkillUiRoot;

#[derive(Component)]
struct SkillCard {
    slot_index: usize,
    skill: SkillId,
}

#[derive(Component)]
struct SkillCooldownText {
    slot_index: usize,
}

#[derive(Component)]
struct HpText;

#[derive(Resource)]
struct SkillSpawnTimer(pub Timer);

#[derive(Resource, Default)]
struct SkillCooldowns {
    slot: [f32; MAX_SKILL_CARDS],
}

pub struct SkillPlugin;

impl Plugin for SkillPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(SkillSpawnTimer(Timer::from_seconds(3.0, TimerMode::Repeating)))
            .init_resource::<SkillCooldowns>()
            .add_systems(OnEnter(GameState::InGame), setup_skill_ui)
            .add_systems(OnExit(GameState::InGame), cleanup_skill_ui)
            .add_systems(
                Update,
                (
                    spawn_other_skills,
                    use_number_key_skills,
                    use_dash_skill_with_ctrl,
                    update_hp_text,
                    update_skill_cooldowns,
                )
                    .in_set(CombatSet),
            );
    }
}

fn setup_skill_ui(mut commands: Commands) {
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
            Text::new("HP"),
            TextFont { font_size: 18.0, ..default() },
            TextColor(Color::WHITE),
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(16.0),
                top: Val::Px(16.0),
                ..default()
            },
        ));

        for i in 0..MAX_SKILL_CARDS {
            parent.spawn((
                SkillCard { slot_index: i, skill: SkillId::Slash },
                Node {
                    width: Val::Px(SKILL_CARD_SIZE),
                    height: Val::Px(SKILL_CARD_SIZE),
                    position_type: PositionType::Absolute,
                    left: Val::Px(16.0 + (SKILL_CARD_SIZE + 10.0) * i as f32),
                    bottom: Val::Px(16.0),
                    ..default()
                },
                BackgroundColor(Color::srgba(0.1, 0.1, 0.1, 0.9)),
            ))
            .with_children(|card| {
                card.spawn((
                    Text::new(""),
                    TextFont { font_size: 14.0, ..default() },
                    TextColor(Color::WHITE),
                    Node {
                        position_type: PositionType::Absolute,
                        left: Val::Px(6.0),
                        top: Val::Px(6.0),
                        ..default()
                    },
                ));

                card.spawn((
                    SkillCooldownText { slot_index: i },
                    Text::new(""),
                    TextFont { font_size: 12.0, ..default() },
                    TextColor(Color::WHITE),
                    Node {
                        position_type: PositionType::Absolute,
                        left: Val::Px(6.0),
                        bottom: Val::Px(6.0),
                        ..default()
                    },
                ));
            });
        }
    });
}

fn cleanup_skill_ui(mut commands: Commands, root_q: Query<Entity, With<SkillUiRoot>>) {
    for e in root_q.iter() {
        commands.entity(e).despawn();
    }
}

fn spawn_other_skills(
    time: Res<Time>,
    mut timer: ResMut<SkillSpawnTimer>,
    mut pool: ResMut<SkillPool>,
    cards_q: Query<&SkillCard>,
    mut commands: Commands,
) {
    timer.0.tick(time.delta());
    if !timer.0.just_finished() {
        return;
    }

    let mut used = [false; MAX_SKILL_CARDS];
    for c in cards_q.iter() {
        if c.slot_index < MAX_SKILL_CARDS {
            used[c.slot_index] = true;
        }
    }

    for (i, occupied) in used.iter().enumerate() {
        if !*occupied {
            let skill = pool.next_non_dash();
            commands.spawn((SkillCard { slot_index: i, skill },));
        }
    }
}

fn use_number_key_skills(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut cooldowns: ResMut<SkillCooldowns>,
    mut cards_q: Query<(Entity, &SkillCard)>,
    mut player_q: Query<(&Transform, &mut PlayerAnimation), With<Player>>,
    mut enemies_q: Query<(Entity, &Transform, &mut Health), With<Enemy>>,
    mut commands: Commands,
    pool: Res<SkillPool>,
    mut vfx_pool: ResMut<VfxPool>,
) {
    let Ok((player_tf, anim)) = player_q.single_mut() else { return; };
    let origin = player_tf.translation.truncate();
    let dir = anim.direction.as_vec2().normalize_or_zero();

    let keys = [KeyCode::Digit1, KeyCode::Digit2, KeyCode::Digit3];

    for (slot, key) in keys.iter().enumerate() {
        if !keyboard.just_pressed(*key) {
            continue;
        }
        if cooldowns.slot[slot] > 0.0 {
            continue;
        }

        let mut used_entity = None;
        let mut skill = None;

        for (e, c) in cards_q.iter_mut() {
            if c.slot_index == slot {
                used_entity = Some(e);
                skill = Some(c.skill);
                break;
            }
        }

        let Some(skill) = skill else { continue; };

        match skill {
            SkillId::Slash => {
                spawn_slash_vfx(&mut commands, Some(&mut vfx_pool), origin, dir);
                skill_slash(origin, dir, &mut enemies_q);
                cooldowns.slot[slot] = pool.def(SkillId::Slash).cooldown;
            }
            SkillId::Dash => {}
        }

        if let Some(e) = used_entity {
            commands.entity(e).despawn();
        }
    }
}

fn use_dash_skill_with_ctrl(
    keyboard: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    mut player_q: Query<(Entity, &mut PlayerDash, &mut PlayerAnimation), With<Player>>,
) {
    let Ok((_e, mut dash, anim)) = player_q.single_mut() else { return; };

    dash.cooldown = (dash.cooldown - time.delta_secs()).max(0.0);

    if keyboard.just_pressed(KeyCode::ControlLeft) && dash.cooldown <= 0.0 {
        let dir_vec = anim.direction.as_vec2();
        let dir = if dir_vec == Vec2::ZERO { Vec2::Y } else { dir_vec };

        dash.is_dashing = true;
        dash.remaining = crate::movement::DASH_DURATION;
        dash.direction = dir;
        dash.cooldown = crate::movement::DASH_COOLDOWN;
    }
}

fn update_hp_text(mut q: Query<&mut Text, With<HpText>>, player_q: Query<&Health, With<Player>>) {
    let Ok(player_hp) = player_q.single() else { return; };
    for mut t in &mut q {
        *t = Text::new(format!("HP: {:.0}/{:.0}", player_hp.current, player_hp.max));
    }
}

fn update_skill_cooldowns(
    time: Res<Time>,
    mut cooldowns: ResMut<SkillCooldowns>,
    cards_q: Query<&SkillCard>,
    mut cd_text_q: Query<(&SkillCooldownText, &mut Text)>,
    pool: Res<SkillPool>,
) {
    let dt = time.delta_secs();
    for i in 0..MAX_SKILL_CARDS {
        cooldowns.slot[i] = (cooldowns.slot[i] - dt).max(0.0);
    }

    for (marker, mut t) in &mut cd_text_q {
        let slot = marker.slot_index;
        let mut label = String::new();

        for c in cards_q.iter() {
            if c.slot_index == slot {
                label.push_str(pool.def(c.skill).name);
                label.push('\n');
                break;
            }
        }

        if cooldowns.slot[slot] > 0.0 {
            label.push_str(&format!("{:.1}s", cooldowns.slot[slot]));
        }

        *t = Text::new(label);
    }
}
