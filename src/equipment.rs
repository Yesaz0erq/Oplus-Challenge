use bevy::prelude::*;
use bevy::input::keyboard::KeyCode;

use crate::state::GameState;
use crate::movement::Player;

/// 装备插件：管理装备数据和装备 UI（按 B 打开）
pub struct EquipmentPlugin;

/// 武器类型：近战 / 远程
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum WeaponKind {
    Melee,
    Ranged,
}

/// 玩家身上的装备集合（现在只做了武器，预留道具 / 协议）
#[derive(Component)]
pub struct EquipmentSet {
    pub weapon_kind: WeaponKind,
    pub weapon_damage: f32,
    /// 普攻冷却（秒）
    pub weapon_attack_cooldown: f32,
    /// 远程弹幕速度
    pub weapon_projectile_speed: f32,
    /// 弹幕生存时间
    pub weapon_projectile_lifetime: f32,
    /// 近战攻击长度
    pub melee_range: f32,
    /// 近战攻击宽度
    pub melee_width: f32,
}

impl Default for EquipmentSet {
    fn default() -> Self {
        // 默认给玩家一把近战武器
        Self {
            weapon_kind: WeaponKind::Melee,
            weapon_damage: 20.0,
            weapon_attack_cooldown: 0.6,
            weapon_projectile_speed: 400.0,
            weapon_projectile_lifetime: 1.0,
            melee_range: 80.0,
            melee_width: 40.0,
        }
    }
}

/// 装备 UI 根节点
#[derive(Component)]
pub struct EquipmentUiRoot;

/// 显示装备信息的 Text
#[derive(Component)]
pub struct EquipmentText;

impl Plugin for EquipmentPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                ensure_default_equipment,
                toggle_equipment_ui,
                update_equipment_ui,
            )
                .run_if(in_state(GameState::InGame)),
        );
    }
}

/// 确保玩家身上有一套默认装备
fn ensure_default_equipment(
    mut commands: Commands,
    query: Query<(Entity, Option<&EquipmentSet>), With<Player>>,
) {
    for (entity, existing) in &query {
        if existing.is_none() {
            commands.entity(entity).insert(EquipmentSet::default());
        }
    }
}

/// 按 B 打开 / 关闭装备 UI
fn toggle_equipment_ui(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut commands: Commands,
    ui_root_q: Query<Entity, With<EquipmentUiRoot>>,
    children_q: Query<&Children>,
    asset_server: Res<AssetServer>,
    player_q: Query<&EquipmentSet, With<Player>>,
) {
    if !keyboard.just_pressed(KeyCode::KeyB) {
        return;
    }

    if let Ok(root) = ui_root_q.single() {
        // 已经打开 -> 关闭（递归删除）
        despawn_tree(root, &mut commands, &children_q);
    } else {
        // 还没打开 -> 打开
        let equip = player_q.single().ok();
        spawn_equipment_ui(&mut commands, &asset_server, equip);
    }
}

/// 生成装备 UI 面板
fn spawn_equipment_ui(
    commands: &mut Commands,
    asset_server: &AssetServer,
    equip: Option<&EquipmentSet>,
) {
    let info = if let Some(e) = equip {
        format!(
            "武器: {:?}\n伤害: {:.0}\n冷却: {:.1}s",
            e.weapon_kind, e.weapon_damage, e.weapon_attack_cooldown
        )
    } else {
        "未装备武器".to_string()
    };

    commands
        .spawn((
            EquipmentUiRoot,
            Node {
                position_type: PositionType::Absolute,
                width: Val::Px(320.0),
                height: Val::Px(160.0),
                left: Val::Percent(50.0),
                top: Val::Percent(50.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(Color::srgba(0.05, 0.05, 0.08, 0.9)),
            BorderColor::all(Color::srgb(0.7, 0.7, 1.0)),
            BorderRadius::all(Val::Px(8.0)),
        ))
        .with_children(|parent| {
            parent.spawn((
                EquipmentText,
                Text::new(info),
                TextFont {
                    font: asset_server.load("fonts/YuFanLixing.otf"),
                    font_size: 20.0,
                    ..default()
                },
                TextColor(Color::WHITE),
            ));
        });
}

/// 更新装备 UI 中的文字信息
fn update_equipment_ui(
    player_q: Query<&EquipmentSet, With<Player>>,
    mut text_q: Query<&mut Text, With<EquipmentText>>,
) {
    let Ok(equip) = player_q.single() else {
        return;
    };
    let Ok(mut text) = text_q.single_mut() else {
        return;
    };

    text.0 = format!(
        "武器: {:?}\n伤害: {:.0}\n冷却: {:.1}s",
        equip.weapon_kind, equip.weapon_damage, equip.weapon_attack_cooldown
    );
}

/// 简单的递归销毁 UI 树
fn despawn_tree(entity: Entity, commands: &mut Commands, children_q: &Query<&Children>) {
    if let Ok(children) = children_q.get(entity) {
        for child in children.iter() {
            despawn_tree(child, commands, children_q);
        }
    }
    commands.entity(entity).despawn();
}
