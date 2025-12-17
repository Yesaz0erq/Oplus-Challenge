use bevy::prelude::*;
use crate::skills_pool::SkillPool;
use crate::skills_pool::SkillId;
use crate::combat_core::{skill_slash, spawn_slash_vfx};

#[derive(Resource, Default)]
pub struct EnemySkillTimer(pub Timer);

pub struct EnemyCombatPlugin;

impl Plugin for EnemyCombatPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<EnemySkillTimer>()
            .add_systems(Update, enemy_cast_skills.run_if(in_state(crate::state::GameState::InGame)));
    }
}

fn enemy_cast_skills(
    time: Res<Time>,
    mut timer: ResMut<EnemySkillTimer>,
    mut enemies_q: Query<(&Transform,), With<crate::enemy::Enemy>>,
    mut enemies_health_q: Query<&mut crate::health::Health, With<crate::enemy::Enemy>>,
    mut player_q: Query<&Transform, With<crate::movement::Player>>,
    mut enemies_for_skill: Query<(Entity, &Transform, &mut crate::health::Health), With<crate::enemy::Enemy>>,
) {
    timer.0.tick(time.delta());
    if !timer.0.just_finished() { return; }

    // 简单示例：每隔一段时间让每个敌人在玩家正前方施放 Slash（演示共享 pool）
    let Ok(player_tf) = player_q.single() else { return; };
    for (_e, enemy_tf, mut hp) in &mut enemies_for_skill {
        // 计算方向向玩家
        let dir = (player_tf.translation.truncate() - enemy_tf.translation.truncate()).normalize_or_zero();
        skill_slash(enemy_tf.translation.truncate(), dir, &mut enemies_for_skill);
        spawn_slash_vfx(&mut Commands::default(), enemy_tf.translation.truncate(), dir); // 注意：Commands::default() 仅示例，实际应使用系统 Commands
    }
}