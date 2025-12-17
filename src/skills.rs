use bevy::prelude::*;

use crate::combat::{skill_slash, spawn_slash_vfx};
use crate::enemy::Enemy; // 這裡改成從 enemy 模組引入
use crate::health::Health;
use crate::input::MovementInput;
use crate::movement::{
    DASH_COOLDOWN, DASH_DURATION, Player, PlayerAnimation, PlayerDash, PlayerDirection,
};
use crate::state::GameState;

/// 技能系统插件
pub struct SkillPlugin;

/// 最多同时存在的技能卡牌数量（包含第一个冲刺技能）
const MAX_SKILL_CARDS: usize = 5;

/// 技能标识（技能池）
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SkillId {
    /// 固定在第一个槽位的冲刺技能
    Dash,
    /// slash：前方长条矩形攻击
    Slash,
}

/// 技能生成计时器（只管 2~5 槽）
#[derive(Resource)]
pub struct SkillSpawnTimer(pub Timer);

/// 技能卡牌组件
#[derive(Component)]
pub struct SkillCard {
    pub id: SkillId,
}

/// 冲刺技能标记（固定第一个技能槽）
#[derive(Component)]
pub struct DashSkillCard;

/// HP 文本组件
#[derive(Component)]
pub struct HpText;

impl Plugin for SkillPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(SkillSpawnTimer(Timer::from_seconds(
            3.0,
            TimerMode::Repeating,
        )))
        // 进入 InGame 时生成技能栏 + HP 文本
        .add_systems(OnEnter(GameState::InGame), setup_skill_ui)
        // 离开 InGame 时清理
        .add_systems(OnExit(GameState::InGame), cleanup_skill_ui)
        // InGame 中更新技能与 HP UI
        .add_systems(
            Update,
            (
                spawn_other_skills,
                use_number_key_skills,
                use_dash_skill_with_ctrl,
                update_hp_text,
                update_skill_cooldowns,
            )
                .run_if(in_state(GameState::InGame)),
        );
    }
}

/// 创建技能 UI：
/// - 第一个技能槽固定为“冲刺（Ctrl）”
/// - 上方显示 HP 文本
fn setup_skill_ui(mut commands: Commands, asset_server: Res<AssetServer>) {
    // HP 文本：在技能栏上方
    commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            bottom: Val::Px(90.0),
            left: Val::Px(20.0),
            ..default()
        },
        Text::new("HP: 0 / 0".to_string()),
        TextFont {
            font: asset_server.load("fonts/YuFanLixing.otf"),
            font_size: 22.0,
            ..default()
        },
        TextColor(Color::WHITE),
        HpText,
    ));

    // 第一个技能槽：冲刺
    spawn_skill_card_at_slot(&mut commands, &asset_server, 0, SkillId::Dash);
}

/// 清理所有技能卡牌和 HP 文本
fn cleanup_skill_ui(
    mut commands: Commands,
    cards: Query<Entity, With<SkillCard>>,
    hp_texts: Query<Entity, With<HpText>>,
) {
    for e in cards.iter() {
        commands.entity(e).despawn();
    }
    for e in hp_texts.iter() {
        commands.entity(e).despawn();
    }
}

/// 辅助：在指定槽位生成一张技能卡
///
/// slot_index: 0~4
fn spawn_skill_card_at_slot(
    commands: &mut Commands,
    asset_server: &AssetServer,
    slot_index: usize,
    id: SkillId,
) {
    // 计算位置：第 0 槽在最左边，其余往右排
    let x_offset = 20.0 + slot_index as f32 * 140.0;

    let (name, is_dash) = match id {
        SkillId::Dash => ("冲刺 (Ctrl)", true),
        SkillId::Slash => ("Slash", false),
    };

    // 文本显示：名字 + 冷却
    let display_text = if is_dash {
        format!("{name}\nCD: Ready")
    } else {
        // 非冲刺技能这里简单写成一次性技能，CD 文案固定为 0.0s
        format!("{name}\nCD: 0.0s")
    };

    let mut entity_commands = commands.spawn((
        // 布局节点
        Node {
            position_type: PositionType::Absolute,
            bottom: Val::Px(20.0),
            left: Val::Px(x_offset),
            width: Val::Px(120.0),
            height: Val::Px(50.0),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            ..default()
        },
        // 背景颜色
        BackgroundColor(Color::srgba(0.1, 0.1, 0.2, 0.9)),
        // 文本组件
        Text::new(display_text),
        TextFont {
            font: asset_server.load("fonts/YuFanLixing.otf"),
            font_size: 18.0,
            ..default()
        },
        TextColor(Color::WHITE),
        SkillCard { id },
    ));

    // 如果是冲刺技能，额外插入 DashSkillCard 标记
    if is_dash {
        entity_commands.insert(DashSkillCard);
    }
}

/// 定期生成非冲刺技能（槽位 1~4）
/// 现在的技能池中只有一个真实技能：Slash
fn spawn_other_skills(
    time: Res<Time>,
    mut timer: ResMut<SkillSpawnTimer>,
    asset_server: Res<AssetServer>,
    mut commands: Commands,
    cards: Query<(Entity, &SkillCard, Option<&DashSkillCard>)>,
) {
    // 驱动计时器
    timer.0.tick(time.delta());
    if !timer.0.just_finished() {
        return;
    }

    // 已存在的非冲刺技能数量（槽 1~4）
    let mut non_dash_count = 0usize;
    for (_, _card, is_dash) in &cards {
        if is_dash.is_none() {
            non_dash_count += 1;
        }
    }

    if non_dash_count >= MAX_SKILL_CARDS - 1 {
        return;
    }

    // 槽位 = 1 + 当前已有的非冲刺技能数
    let slot_index = 1 + non_dash_count;

    // 唯一的非冲刺技能：Slash
    spawn_skill_card_at_slot(&mut commands, &asset_server, slot_index, SkillId::Slash);
}

/// 按数字键 1~5 使用对应槽位的技能：
/// - 1 -> 冲刺（也可以用 Ctrl）
/// - 2~5 -> 使用后销毁对应技能卡，并重新排列剩余卡牌的位置；
///   如果是 Slash，则调用公共技能 `skill_slash` + 特效 `spawn_slash_vfx`。
fn use_number_key_skills(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut commands: Commands,
    cards: Query<(Entity, &SkillCard, Option<&DashSkillCard>)>,
    mut node_q: Query<&mut Node>,
    movement: Res<MovementInput>,
    mut player_dash_q: Query<&mut PlayerDash, With<Player>>,
    player_tf_q: Query<&Transform, With<Player>>,
    mut enemies_q: Query<(Entity, &Transform, &mut Health), With<Enemy>>,
    player_anim_q: Query<&PlayerAnimation, With<Player>>,
) {
    let index = if keyboard.just_pressed(KeyCode::Digit1) {
        Some(0)
    } else if keyboard.just_pressed(KeyCode::Digit2) {
        Some(1)
    } else if keyboard.just_pressed(KeyCode::Digit3) {
        Some(2)
    } else if keyboard.just_pressed(KeyCode::Digit4) {
        Some(3)
    } else if keyboard.just_pressed(KeyCode::Digit5) {
        Some(4)
    } else {
        None
    };

    let Some(target_index) = index else {
        return;
    };

    // 收集 dash 和非 dash 卡牌
    let mut non_dash_entities: Vec<Entity> = Vec::new();
    let mut has_dash = false;

    for (entity, card, is_dash) in &cards {
        if is_dash.is_some() && matches!(card.id, SkillId::Dash) {
            has_dash = true;
        } else {
            non_dash_entities.push(entity);
        }
    }

    // 槽 0：冲刺技能
    if target_index == 0 {
        if has_dash {
            trigger_dash_skill(&movement, &mut player_dash_q);
        }
        return;
    }

    // 槽 1~4 对应非冲刺技能
    let non_dash_idx = target_index - 1;
    if non_dash_idx >= non_dash_entities.len() {
        return;
    }

    let entity_to_remove = non_dash_entities[non_dash_idx];

    // 找到这张卡对应的 SkillId
    let Ok((_e, card, _is_dash)) = cards.get(entity_to_remove) else {
        return;
    };

    // 使用技能：根据 SkillId 调用对应技能逻辑
    match card.id {
        SkillId::Slash => {
            // 玩家位置
            let Ok(player_tf) = player_tf_q.single() else {
                return;
            };

            // 技能方向：
            // 1. 有移动输入就用当前输入方向
            // 2. 没有输入时，用动画里记录的朝向
            let dir = if movement.0 != Vec2::ZERO {
                movement.0.normalize()
            } else if let Ok(anim) = player_anim_q.single() {
                anim.direction.as_vec2()
            } else {
                // 兜底方向（比如没有找到动画组件）：向下
                Vec2::new(0.0, -1.0)
            };

            // 1. 公共技能池：沿朝向的矩形伤害
            skill_slash(player_tf.translation.truncate(), dir, &mut enemies_q);

            // 2. 公共特效：沿朝向旋转的矩形光效
            spawn_slash_vfx(&mut commands, player_tf.translation.truncate(), dir);
        }
        SkillId::Dash => {
            // 理论上不会走到这里（Dash 在槽 0），但防御性处理一下
            trigger_dash_skill(&movement, &mut player_dash_q);
        }
    }

    // 使用后销毁该卡牌（Slash 是一次性技能）
    commands.entity(entity_to_remove).despawn();

    // 从列表中移除
    non_dash_entities.remove(non_dash_idx);

    // 重新排列剩余非冲刺技能的位置，使它们从槽 1 开始连续
    for (i, e) in non_dash_entities.iter().enumerate() {
        if let Ok(mut node) = node_q.get_mut(*e) {
            node.left = Val::Px(20.0 + (i as f32 + 1.0) * 140.0);
            node.bottom = Val::Px(20.0);
        }
    }
}

/// Ctrl 触发冲刺技能
fn use_dash_skill_with_ctrl(
    keyboard: Res<ButtonInput<KeyCode>>,
    movement: Res<MovementInput>,
    mut player_dash_q: Query<&mut PlayerDash, With<Player>>,
) {
    // Ctrl 触发
    if !(keyboard.just_pressed(KeyCode::ControlLeft)
        || keyboard.just_pressed(KeyCode::ControlRight))
    {
        return;
    }

    trigger_dash_skill(&movement, &mut player_dash_q);
}

/// 实际触发冲刺的逻辑：
/// - 只有在有移动方向时才冲刺
/// - 如果正在冲刺或在冷却中则不触发
fn trigger_dash_skill(
    movement: &MovementInput,
    player_dash_q: &mut Query<&mut PlayerDash, With<Player>>,
) {
    let input_dir = movement.0;
    if input_dir == Vec2::ZERO {
        return;
    }

    let Ok(mut dash) = player_dash_q.single_mut() else {
        return;
    };

    if dash.is_dashing || dash.cooldown > 0.0 {
        return;
    }

    dash.is_dashing = true;
    dash.remaining = DASH_DURATION;
    dash.cooldown = DASH_COOLDOWN;
    dash.direction = input_dir.normalize();

    // 冲刺期间的免疫在别的伤害系统里检查 dash.is_dashing 即可
}

/// 更新 HP 文本：显示 `HP: current / max`
fn update_hp_text(
    player_health_q: Query<&Health, With<Player>>,
    mut hp_text_q: Query<&mut Text, With<HpText>>,
) {
    let Ok(health) = player_health_q.single() else {
        return;
    };
    let Ok(mut text) = hp_text_q.single_mut() else {
        return;
    };

    text.0 = format!("HP: {:.0} / {:.0}", health.current, health.max);
}

/// 更新技能冷却显示：
/// - 冲刺：显示剩余冷却时间或 Ready
/// - 非冲刺技能：固定 `CD: 0.0s`（一次性技能，用掉就消失）
fn update_skill_cooldowns(
    player_dash_q: Query<&PlayerDash, With<Player>>,
    mut dash_text_q: Query<&mut Text, With<DashSkillCard>>,
) {
    let Ok(dash) = player_dash_q.single() else {
        return;
    };
    let Ok(mut text) = dash_text_q.single_mut() else {
        return;
    };

    let cd = dash.cooldown.max(0.0);
    if cd > 0.0 {
        text.0 = format!("冲刺 (Ctrl)\nCD: {:.1}s", cd);
    } else {
        text.0 = "冲刺 (Ctrl)\nCD: Ready".to_string();
    }
}
