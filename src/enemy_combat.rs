use bevy::prelude::*;

use crate::combat_core::{spawn_slash_vfx, skill_slash_on_player, CombatSet, VfxPool};
use crate::enemy::Enemy;
use crate::health::Health;
use crate::movement::Player;
use crate::skills_pool::{SkillId, SkillPool};
use crate::state::GameState;

#[derive(Resource)]
struct EnemyCastTimer(Timer);

pub struct EnemyCombatPlugin;

impl Plugin for EnemyCombatPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(EnemyCastTimer(Timer::from_seconds(1.2, TimerMode::Repeating)))
            .add_systems(Update, enemy_cast_skill.in_set(CombatSet).run_if(in_state(GameState::InGame)));
    }
}

fn enemy_cast_skill(
    time: Res<Time>,
    mut timer: ResMut<EnemyCastTimer>,
    mut pool: ResMut<SkillPool>,
    mut commands: Commands,
    enemies_q: Query<&Transform, With<Enemy>>,
    mut player_q: Query<(&Transform, &mut Health), With<Player>>,
    mut vfx_pool: ResMut<VfxPool>,
) {
    timer.0.tick(time.delta());
    if !timer.0.just_finished() {
        return;
    }

    let Ok((player_tf, mut player_hp)) = player_q.single_mut() else { return; };
    let player_pos = player_tf.translation.truncate();

    let mut best_enemy_pos = None;
    let mut best_dist = f32::MAX;

    for tf in enemies_q.iter() {
        let pos = tf.translation.truncate();
        let dist = pos.distance(player_pos);
        if dist < best_dist {
            best_dist = dist;
            best_enemy_pos = Some(pos);
        }
    }

    let Some(enemy_pos) = best_enemy_pos else { return; };
    if best_dist > 160.0 {
        return;
    }

    let skill = pool.next_non_dash();
    match skill {
        SkillId::Slash => {
            let dir = (player_pos - enemy_pos).normalize_or_zero();
            spawn_slash_vfx(&mut commands, Some(&mut vfx_pool), enemy_pos, dir);
            skill_slash_on_player(enemy_pos, dir, player_pos, &mut player_hp);
        }
        SkillId::Dash => {}
    }
}
