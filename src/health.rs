use crate::movement::Player;
use crate::state::GameState;
use bevy::prelude::*;

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

pub struct HealthPlugin;

impl Plugin for HealthPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            check_player_death.run_if(in_state(GameState::InGame)),
        );
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
