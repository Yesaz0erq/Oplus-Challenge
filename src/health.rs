use bevy::prelude::*;

use crate::movement::Player;
use crate::state::GameState;

pub const PLAYER_HIT_IFRAMES_SECS: f32 = 0.5;

#[derive(Component)]
pub struct Health {
    pub current: f32,
    pub max: f32,
}

impl Health {
    pub fn new(max: f32) -> Self {
        Self { current: max, max }
    }
}

#[derive(Component, Default)]
pub struct PlayerHitIFrames {
    pub remaining: f32,
}

pub fn try_damage_player(
    health: &mut Health,
    iframes: &mut PlayerHitIFrames,
    damage: f32,
) -> bool {
    if damage <= 0.0 || iframes.remaining > 0.0 {
        return false;
    }
    health.current -= damage;
    iframes.remaining = PLAYER_HIT_IFRAMES_SECS;
    true
}

pub struct HealthPlugin;

impl Plugin for HealthPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (tick_player_hit_iframes, check_player_death).run_if(in_state(GameState::InGame)),
        );
    }
}

fn tick_player_hit_iframes(time: Res<Time>, mut q: Query<&mut PlayerHitIFrames, With<Player>>) {
    let dt = time.delta_secs();
    for mut iframes in &mut q {
        iframes.remaining = (iframes.remaining - dt).max(0.0);
    }
}

fn check_player_death(
    mut commands: Commands,
    mut next_state: ResMut<NextState<GameState>>,
    query: Query<(Entity, &Health), With<Player>>,
) {
    if let Some((entity, health)) = query.iter().next() {
        if health.current <= 0.0 {
            commands.entity(entity).despawn();
            next_state.set(GameState::GameOver);
        }
    }
}
