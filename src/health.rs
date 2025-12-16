use bevy::prelude::*;

use crate::state::GameState;
use crate::movement::Player;

/// 通用生命组件：挂在玩家和敌人身上
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

/// 玩家死亡 -> 切到 GameOver
fn check_player_death(
    mut commands: Commands,
    mut next_state: ResMut<NextState<GameState>>,
    query: Query<(Entity, &Health), With<Player>>,
) {
    // 0.17 里我们用 iter().next() 取第一个玩家
    if let Some((entity, health)) = query.iter().next() {
        if health.current <= 0.0 {
            // 玩家死了，删掉玩家实体，进入 GameOver 场景
            commands.entity(entity).despawn();
            next_state.set(GameState::GameOver);
        }
    }
}
