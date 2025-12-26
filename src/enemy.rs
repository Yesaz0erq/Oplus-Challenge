use bevy::prelude::*;

use crate::health::Health;
use crate::movement::Player;
use crate::state::GameState;

#[derive(Component)]
pub struct Enemy;

#[derive(Component)]
pub struct EnemySpeed(pub f32);

#[derive(Component)]
pub struct EnemyDamage(pub f32);

#[derive(Resource)]
struct EnemySpawnTimer(pub Timer);

impl Default for EnemySpawnTimer {
    fn default() -> Self {
        // 你也可以把 1.0 改成你想要的刷怪间隔（秒）
        Self(Timer::from_seconds(1.0, TimerMode::Repeating))
    }
}

pub struct EnemyPlugin;

impl Plugin for EnemyPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<EnemySpawnTimer>().add_systems(
            Update,
            (
                spawn_enemies_periodically.run_if(in_state(GameState::InGame)),
                move_enemies_towards_player.run_if(in_state(GameState::InGame)),
                damage_player_on_contact.run_if(in_state(GameState::InGame)),
            ),
        );
    }
}

fn spawn_enemies_periodically(
    mut commands: Commands,
    time: Res<Time>,
    mut timer: ResMut<EnemySpawnTimer>,
    player_q: Query<&Transform, With<Player>>,
    asset_server: Res<AssetServer>,
) {
    let Ok(player_tf) = player_q.single() else { return; };
    let ppos = player_tf.translation.truncate();

    timer.0.tick(time.delta());
    if !timer.0.just_finished() {
        return;
    }

    // 每次刷 3 个，围着玩家一圈
    for i in 0..1 {
        let ang = (i as f32) * std::f32::consts::TAU / 1.0;
        let offset = Vec2::new(ang.cos(), ang.sin()) * 200.0;
        let pos = ppos + offset;

        let texture: Handle<Image> = asset_server.load("enemy.png");
        let mut sprite = Sprite::from_image(texture);
        sprite.custom_size = Some(Vec2::splat(28.0));

        commands.spawn((
            sprite,
            Transform::from_translation(pos.extend(10.0)),
            Enemy,
            EnemySpeed(60.0 + (i as f32) * 8.0),
            EnemyDamage(8.0 + (i as f32) * 1.5),
            Health { current: 40.0, max: 40.0 },
        ));
    }
}

fn move_enemies_towards_player(
    time: Res<Time>,
    player_q: Query<&Transform, (With<Player>, Without<Enemy>)>,
    mut enemy_q: Query<(&mut Transform, &EnemySpeed), (With<Enemy>, Without<Player>)>,
) {
    let Ok(player_tf) = player_q.single() else { return; };
    let ppos = player_tf.translation.truncate();
    let dt = time.delta_secs();

    for (mut tf, speed) in enemy_q.iter_mut() {
        let pos = tf.translation.truncate();
        let dir = (ppos - pos).normalize_or_zero();
        let delta = dir * speed.0 * dt;

        tf.translation.x += delta.x;
        tf.translation.y += delta.y;
    }
}

fn damage_player_on_contact(
    mut player_q: Query<(&mut Health, &Transform), (With<Player>, Without<Enemy>)>,
    enemies_q: Query<(&Transform, &EnemyDamage), (With<Enemy>, Without<Player>)>,
) {
    let Ok((mut player_hp, player_tf)) = player_q.single_mut() else { return; };
    let ppos = player_tf.translation.truncate();

    for (tf, dmg) in enemies_q.iter() {
        let dist = tf.translation.truncate().distance(ppos);
        if dist <= 1.0 {
            player_hp.current -= dmg.0;
        }
    }
}