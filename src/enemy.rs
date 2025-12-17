use bevy::prelude::*;
use rand::prelude::*;

use crate::health::Health;
use crate::movement::Player;
use crate::state::GameState;

/// 敌人标记组件: 所有敌对单位都加上这个
#[derive(Component)]
pub struct Enemy;

/// 敌人移动速度
#[derive(Component)]
pub struct EnemyMoveSpeed(pub f32);

/// 接触伤害配置
#[derive(Component)]
pub struct ContactDamage {
    pub damage_per_hit: f32,
}

/// 敌人对玩家接触伤害的冷却
#[derive(Component)]
pub struct ContactCooldown {
    /// 距离下一次允许造成伤害的剩余时间（秒）
    pub remaining: f32,
    /// 每次攻击后的冷却时间（秒）
    pub cooldown: f32,
}

/// 敌人生成计时器
#[derive(Resource)]
pub struct EnemySpawnTimer(pub Timer);

pub struct EnemyPlugin;

impl Plugin for EnemyPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(EnemySpawnTimer(Timer::from_seconds(
            2.5,
            TimerMode::Repeating,
        )))
        .add_systems(
            Update,
            (
                spawn_enemies_around_player,
                move_enemies_towards_player,
                apply_contact_damage_to_player, // ✅ 修正：用正确的函数名
            )
                .run_if(in_state(GameState::InGame)),
        );
    }
}

/// 在玩家周围随机刷怪
fn spawn_enemies_around_player(
    mut commands: Commands,
    time: Res<Time>,
    mut spawn_timer: ResMut<EnemySpawnTimer>,
    player_q: Query<&Transform, With<Player>>,
    asset_server: Res<AssetServer>,
) {
    let Ok(player_tf) = player_q.single() else {
        return;
    };

    // 驱动生成计时器
    spawn_timer.0.tick(time.delta());
    if !spawn_timer.0.just_finished() {
        return;
    }

    // 在玩家周围随机一个方向刷怪
    let mut rng = thread_rng();
    let radius = 500.0;
    let angle: f32 = rng.gen_range(0.0..std::f32::consts::TAU);
    let offset = Vec2::new(angle.cos(), angle.sin()) * radius;
    let spawn_pos = player_tf.translation.truncate() + offset;

    // 敌人贴图
    let texture: Handle<Image> = asset_server.load("enemy.png");

    let mut sprite = Sprite::from_image(texture);
    sprite.custom_size = Some(Vec2::splat(40.0));
    sprite.color = Color::srgb(0.9, 0.3, 0.3);

    commands.spawn((
        sprite,
        Transform::from_xyz(spawn_pos.x, spawn_pos.y, 5.0),
        Enemy,
        EnemyMoveSpeed(80.0), // 缓慢靠近玩家
        ContactDamage {
            damage_per_hit: 8.0,
        },
        ContactCooldown {
            remaining: 0.0,
            cooldown: 0.8, // 每 0.8 秒最多打一次
        },
        Health {
            current: 100.0,
            max: 100.0,
        },
    ));
}

/// 敌人缓慢向玩家移动
fn move_enemies_towards_player(
    time: Res<Time>,
    // 玩家：有 Player，且明确「没有 Enemy」
    player_q: Query<&Transform, (With<Player>, Without<Enemy>)>,
    // 敌人：有 Enemy，且明确「没有 Player」
    mut enemies_q: Query<(&mut Transform, &EnemyMoveSpeed), (With<Enemy>, Without<Player>)>,
) {
    let Ok(player_tf) = player_q.single() else {
        return;
    };

    let dt = time.delta_secs();

    for (mut enemy_tf, EnemyMoveSpeed(speed)) in &mut enemies_q {
        let dir = (player_tf.translation.truncate() - enemy_tf.translation.truncate())
            .normalize_or_zero();
        enemy_tf.translation += dir.extend(0.0) * *speed * dt;
    }
}

/// 敌人靠近玩家时造成接触伤害
fn apply_contact_damage_to_player(
    time: Res<Time>,
    mut player_q: Query<(&Transform, &mut Health), With<Player>>,
    mut enemies_q: Query<(&Transform, &ContactDamage, &mut ContactCooldown), With<Enemy>>,
) {
    let dt = time.delta_secs();

    let Ok((player_tf, mut player_hp)) = player_q.single_mut() else {
        return;
    };

    for (
        enemy_tf,
        ContactDamage {
            damage_per_hit: dmg,
        },
        mut cd,
    ) in &mut enemies_q
    {
        // 冷却计时
        if cd.remaining > 0.0 {
            cd.remaining -= dt;
            continue;
        }

        let dist = player_tf
            .translation
            .truncate()
            .distance(enemy_tf.translation.truncate());

        // 接触范围：可以根据角色大小再调整
        if dist < 32.0 {
            player_hp.current -= *dmg;
            cd.remaining = cd.cooldown;
        }
    }
}
