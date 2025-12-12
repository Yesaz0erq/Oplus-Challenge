use bevy::input::keyboard::KeyCode;
use bevy::prelude::*;
use bevy::ui::{FocusPolicy, ZIndex};
use std::collections::HashMap;

use crate::movement::Player;
use crate::state::GameState;

/// 装备插件：管理装备数据 + 装备/背包 UI
pub struct EquipmentPlugin;

/// UI 打开键（可配置，默认 B）
/// 你可以在 main.rs 里 insert_resource 覆盖它。
#[derive(Resource)]
pub struct EquipmentUiConfig {
    pub toggle_key: KeyCode,
}
impl Default for EquipmentUiConfig {
    fn default() -> Self {
        Self {
            toggle_key: KeyCode::KeyB,
        }
    }
}

/// 武器类型：近战 / 远程
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum WeaponKind {
    Melee,
    Ranged,
}

/// 玩家身上的装备集合（战斗系统依赖这个组件！）
#[derive(Component, Clone)]
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
        // 默认给玩家一把近战武器（与你当前仓库默认值对齐）
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

/// 物品 ID（目前只做武器，你后续可扩展为 Armor / Consumable 等）
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum ItemId {
    RustySword,
    MagicWand,
    HunterBow,
}

impl Default for ItemId {
    fn default() -> Self {
        ItemId::RustySword
    }
}

impl ItemId {
    pub fn display_name(self) -> &'static str {
        match self {
            ItemId::RustySword => "生锈短剑",
            ItemId::MagicWand => "法杖",
            ItemId::HunterBow => "猎弓",
        }
    }
}

/// 武器定义（用于从 ItemId 计算 EquipmentSet）
#[derive(Clone)]
pub struct WeaponDef {
    pub name: &'static str,
    pub kind: WeaponKind,
    pub damage: f32,
    pub cooldown: f32,
    pub projectile_speed: f32,
    pub projectile_lifetime: f32,
    pub melee_range: f32,
    pub melee_width: f32,
}

#[derive(Resource)]
pub struct ItemDatabase {
    weapons: HashMap<ItemId, WeaponDef>,
}

impl Default for ItemDatabase {
    fn default() -> Self {
        let mut weapons = HashMap::new();

        // 默认近战（与你现有默认装备数值对齐）
        weapons.insert(
            ItemId::RustySword,
            WeaponDef {
                name: "生锈短剑",
                kind: WeaponKind::Melee,
                damage: 20.0,
                cooldown: 0.6,
                projectile_speed: 400.0,
                projectile_lifetime: 1.0,
                melee_range: 80.0,
                melee_width: 40.0,
            },
        );

        // 远程法杖
        weapons.insert(
            ItemId::MagicWand,
            WeaponDef {
                name: "法杖",
                kind: WeaponKind::Ranged,
                damage: 14.0,
                cooldown: 0.35,
                projectile_speed: 520.0,
                projectile_lifetime: 1.2,
                melee_range: 60.0,
                melee_width: 30.0,
            },
        );

        // 远程弓
        weapons.insert(
            ItemId::HunterBow,
            WeaponDef {
                name: "猎弓",
                kind: WeaponKind::Ranged,
                damage: 18.0,
                cooldown: 0.55,
                projectile_speed: 650.0,
                projectile_lifetime: 1.0,
                melee_range: 60.0,
                melee_width: 30.0,
            },
        );

        Self { weapons }
    }
}

impl ItemDatabase {
    pub fn weapon(&self, id: ItemId) -> Option<&WeaponDef> {
        self.weapons.get(&id)
    }
}

impl EquipmentSet {
    fn from_weapon(def: &WeaponDef) -> Self {
        Self {
            weapon_kind: def.kind,
            weapon_damage: def.damage,
            weapon_attack_cooldown: def.cooldown,
            weapon_projectile_speed: def.projectile_speed,
            weapon_projectile_lifetime: def.projectile_lifetime,
            melee_range: def.melee_range,
            melee_width: def.melee_width,
        }
    }
}

/// 背包堆叠
#[derive(Clone, Copy, Debug)]
pub struct ItemStack {
    pub id: ItemId,
    pub count: u32,
}

/// 玩家背包（组件，挂 Player）
#[derive(Component, Default)]
pub struct Inventory {
    pub items: Vec<ItemStack>,
}

impl Inventory {
    pub fn add_one(&mut self, id: ItemId) {
        if let Some(s) = self.items.iter_mut().find(|s| s.id == id) {
            s.count += 1;
        } else {
            self.items.push(ItemStack { id, count: 1 });
        }
    }

    pub fn remove_one(&mut self, id: ItemId) -> bool {
        if let Some(i) = self.items.iter().position(|s| s.id == id && s.count > 0) {
            let mut s = self.items[i];
            s.count -= 1;
            if s.count == 0 {
                self.items.remove(i);
            } else {
                self.items[i] = s;
            }
            true
        } else {
            false
        }
    }
}

/// 已装备信息（目前只做武器槽）
#[derive(Component)]
pub struct EquippedItems {
    pub weapon: ItemId,
}

impl Default for EquippedItems {
    fn default() -> Self {
        Self {
            weapon: ItemId::default(),
        }
    }
}

/// UI 根节点
#[derive(Component)]
pub struct EquipmentUiRoot;

/// 装备 UI 的按钮标记（避免影响其他 UI）
#[derive(Component)]
struct EquipmentUiButton;

/// 背包按钮数据
#[derive(Component)]
struct InventoryItemButton {
    item_id: ItemId,
}

/// 装备消息：装备一把武器（Bevy 0.17 使用 Message）
#[derive(Message, Clone, Copy, Debug)]
struct EquipWeaponMsg {
    item_id: ItemId,
}

/// UI 需要重建（背包/装备变化后）
#[derive(Resource, Default)]
struct EquipmentUiDirty(bool);

impl Plugin for EquipmentPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<EquipmentUiConfig>()
            .init_resource::<ItemDatabase>()
            .init_resource::<EquipmentUiDirty>()
            .add_message::<EquipWeaponMsg>()
            // 用单独 add_systems + run_if，避免 tuple run_if 的兼容问题
            .add_systems(
                Update,
                ensure_player_inventory_and_equipment.run_if(in_state(GameState::InGame)),
            )
            .add_systems(Update, toggle_equipment_ui.run_if(in_state(GameState::InGame)))
            .add_systems(
                Update,
                handle_equipment_ui_buttons.run_if(in_state(GameState::InGame)),
            )
            .add_systems(
                Update,
                apply_equip_weapon_messages.run_if(in_state(GameState::InGame)),
            )
            .add_systems(
                Update,
                rebuild_equipment_ui_when_dirty.run_if(in_state(GameState::InGame)),
            );
    }
}

/// 确保玩家身上有：Inventory + EquippedItems + EquipmentSet
fn ensure_player_inventory_and_equipment(
    mut commands: Commands,
    db: Res<ItemDatabase>,
    q: Query<
        (
            Entity,
            Option<&Inventory>,
            Option<&EquippedItems>,
            Option<&EquipmentSet>,
        ),
        With<Player>,
    >,
) {
    for (e, inv, equipped, equip_set) in &q {
        if inv.is_none() {
            let mut inv = Inventory::default();
            // 初始背包：给两把可换的武器
            inv.add_one(ItemId::MagicWand);
            inv.add_one(ItemId::HunterBow);
            commands.entity(e).insert(inv);
        }

        let weapon_id = equipped.map(|x| x.weapon).unwrap_or_default();

        if equipped.is_none() {
            commands.entity(e).insert(EquippedItems { weapon: weapon_id });
        }

        if equip_set.is_none() {
            if let Some(def) = db.weapon(weapon_id) {
                commands.entity(e).insert(EquipmentSet::from_weapon(def));
            } else {
                commands.entity(e).insert(EquipmentSet::default());
            }
        }
    }
}

/// 按配置键打开/关闭装备 UI
fn toggle_equipment_ui(
    keyboard: Res<ButtonInput<KeyCode>>,
    cfg: Res<EquipmentUiConfig>,
    mut dirty: ResMut<EquipmentUiDirty>,
    mut commands: Commands,
    ui_root_q: Query<Entity, With<EquipmentUiRoot>>,
    children_q: Query<&Children>,
    asset_server: Res<AssetServer>,
    db: Res<ItemDatabase>,
    player_q: Query<(&EquipmentSet, &EquippedItems, &Inventory), With<Player>>,
) {
    if !keyboard.just_pressed(cfg.toggle_key) {
        return;
    }

    // ✅ Bevy 0.17：Query 用 single()/single_mut() :contentReference[oaicite:1]{index=1}
    if let Ok(root) = ui_root_q.single() {
        despawn_tree(root, &mut commands, &children_q);
        return;
    }

    let Ok((equip, equipped, inv)) = player_q.single() else {
        return;
    };

    dirty.0 = false;
    spawn_equipment_ui(
        &mut commands,
        &asset_server,
        &db,
        equip,
        equipped,
        inv,
        cfg.toggle_key,
    );
}

/// 处理装备 UI 按钮：hover 变色、点击写入装备消息
fn handle_equipment_ui_buttons(
    mut interactions: Query<
        (
            &Interaction,
            &mut BackgroundColor,
            Option<&InventoryItemButton>,
        ),
        (Changed<Interaction>, With<Button>, With<EquipmentUiButton>),
    >,
    mut writer: MessageWriter<EquipWeaponMsg>,
) {
    for (interaction, mut color, item_btn) in &mut interactions {
        match *interaction {
            Interaction::Pressed => {
                color.0 = Color::srgb(0.8, 0.8, 1.0);
                if let Some(btn) = item_btn {
                    writer.write(EquipWeaponMsg { item_id: btn.item_id });
                }
            }
            Interaction::Hovered => {
                color.0 = Color::srgb(0.6, 0.6, 0.8);
            }
            Interaction::None => {
                color.0 = Color::srgb(0.25, 0.25, 0.35);
            }
        }
    }
}

/// 应用“装备武器”消息：从背包拿出新武器 -> 旧武器回包 -> 更新 EquipmentSet
fn apply_equip_weapon_messages(
    mut reader: MessageReader<EquipWeaponMsg>,
    db: Res<ItemDatabase>,
    mut dirty: ResMut<EquipmentUiDirty>,
    mut q: Query<(&mut Inventory, &mut EquippedItems, &mut EquipmentSet), With<Player>>,
) {
    let Ok((mut inv, mut equipped, mut equip_set)) = q.single_mut() else {
        return;
    };

    for m in reader.read() {
        let new_id = m.item_id;

        // 已装备同一把就忽略
        if new_id == equipped.weapon {
            continue;
        }

        // 背包没有就忽略
        if !inv.remove_one(new_id) {
            continue;
        }

        // 旧武器回包
        inv.add_one(equipped.weapon);

        // 更新装备与战斗用参数
        equipped.weapon = new_id;
        if let Some(def) = db.weapon(new_id) {
            *equip_set = EquipmentSet::from_weapon(def);
        }

        dirty.0 = true;
    }
}

/// 当背包/装备变化且 UI 打开时，重建 UI
fn rebuild_equipment_ui_when_dirty(
    mut dirty: ResMut<EquipmentUiDirty>,
    mut commands: Commands,
    ui_root_q: Query<Entity, With<EquipmentUiRoot>>,
    children_q: Query<&Children>,
    asset_server: Res<AssetServer>,
    db: Res<ItemDatabase>,
    cfg: Res<EquipmentUiConfig>,
    player_q: Query<(&EquipmentSet, &EquippedItems, &Inventory), With<Player>>,
) {
    if !dirty.0 {
        return;
    }

    let Ok(root) = ui_root_q.single() else {
        dirty.0 = false;
        return;
    };

    let Ok((equip, equipped, inv)) = player_q.single() else {
        dirty.0 = false;
        return;
    };

    despawn_tree(root, &mut commands, &children_q);
    spawn_equipment_ui(
        &mut commands,
        &asset_server,
        &db,
        equip,
        equipped,
        inv,
        cfg.toggle_key,
    );

    dirty.0 = false;
}

/// 生成装备/背包 UI 面板
fn spawn_equipment_ui(
    commands: &mut Commands,
    asset_server: &AssetServer,
    db: &ItemDatabase,
    equip: &EquipmentSet,
    equipped: &EquippedItems,
    inv: &Inventory,
    toggle_key: KeyCode,
) {
    let font = asset_server.load("fonts/YuFanLixing.otf");

    let equipped_name = equipped.weapon.display_name();
    let equipped_kind = format!("{:?}", equip.weapon_kind);
    let equipped_info = format!(
        "武器：{equipped_name}\n类型：{equipped_kind}\n伤害：{:.0}\n冷却：{:.2}s",
        equip.weapon_damage, equip.weapon_attack_cooldown
    );

    commands
        .spawn((
            EquipmentUiRoot,
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                position_type: PositionType::Absolute,
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            FocusPolicy::Block,
            ZIndex(9),
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.55)),
        ))
        .with_children(|overlay| {
            overlay
                .spawn((
                    Node {
                        width: Val::Px(720.0),
                        height: Val::Px(420.0),
                        flex_direction: FlexDirection::Column,
                        row_gap: Val::Px(12.0),
                        padding: UiRect::all(Val::Px(14.0)),
                        border: UiRect::all(Val::Px(2.0)),
                        ..default()
                    },
                    BackgroundColor(Color::srgba(0.12, 0.12, 0.16, 0.95)),
                    BorderColor::all(Color::srgb(0.7, 0.7, 1.0)),
                    BorderRadius::all(Val::Px(10.0)),
                ))
                .with_children(|panel| {
                    // 顶栏
                    panel
                        .spawn(Node {
                            width: Val::Percent(100.0),
                            height: Val::Auto,
                            flex_direction: FlexDirection::Row,
                            justify_content: JustifyContent::SpaceBetween,
                            align_items: AlignItems::Center,
                            ..default()
                        })
                        .with_children(|row| {
                            row.spawn((
                                Text::new("装备 / 背包".to_string()),
                                TextFont {
                                    font: font.clone(),
                                    font_size: 26.0,
                                    ..default()
                                },
                                TextColor(Color::WHITE),
                            ));

                            row.spawn((
                                Text::new(format!("按 {:?} 关闭", toggle_key)),
                                TextFont {
                                    font: font.clone(),
                                    font_size: 16.0,
                                    ..default()
                                },
                                TextColor(Color::srgb(0.9, 0.9, 0.3)),
                            ));
                        });

                    // 内容两列
                    panel
                        .spawn(Node {
                            width: Val::Percent(100.0),
                            height: Val::Percent(100.0),
                            flex_direction: FlexDirection::Row,
                            column_gap: Val::Px(14.0),
                            ..default()
                        })
                        .with_children(|content| {
                            // 左：已装备
                            content
                                .spawn(Node {
                                    width: Val::Percent(45.0),
                                    height: Val::Percent(100.0),
                                    flex_direction: FlexDirection::Column,
                                    row_gap: Val::Px(10.0),
                                    ..default()
                                })
                                .with_children(|left| {
                                    left.spawn((
                                        Text::new("已装备".to_string()),
                                        TextFont {
                                            font: font.clone(),
                                            font_size: 20.0,
                                            ..default()
                                        },
                                        TextColor(Color::WHITE),
                                    ));

                                    left.spawn((
                                        Text::new(equipped_info),
                                        TextFont {
                                            font: font.clone(),
                                            font_size: 18.0,
                                            ..default()
                                        },
                                        TextColor(Color::srgb(0.92, 0.92, 0.92)),
                                    ));

                                    left.spawn((
                                        Text::new(
                                            "提示：点击右侧背包的武器即可替换（旧武器会回到背包）"
                                                .to_string(),
                                        ),
                                        TextFont {
                                            font: font.clone(),
                                            font_size: 14.0,
                                            ..default()
                                        },
                                        TextColor(Color::srgb(0.75, 0.75, 0.9)),
                                    ));
                                });

                            // 右：背包
                            content
                                .spawn(Node {
                                    width: Val::Percent(55.0),
                                    height: Val::Percent(100.0),
                                    flex_direction: FlexDirection::Column,
                                    row_gap: Val::Px(10.0),
                                    ..default()
                                })
                                .with_children(|right| {
                                    right.spawn((
                                        Text::new("背包（Inventory）".to_string()),
                                        TextFont {
                                            font: font.clone(),
                                            font_size: 20.0,
                                            ..default()
                                        },
                                        TextColor(Color::WHITE),
                                    ));

                                    right
                                        .spawn(Node {
                                            width: Val::Percent(100.0),
                                            height: Val::Percent(100.0),
                                            flex_direction: FlexDirection::Column,
                                            row_gap: Val::Px(8.0),
                                            padding: UiRect::all(Val::Px(8.0)),
                                            ..default()
                                        })
                                        .with_children(|list| {
                                            let mut items = inv.items.clone();
                                            items.sort_by_key(|s| s.id as u32);

                                            if items.is_empty() {
                                                list.spawn((
                                                    Text::new("（空）".to_string()),
                                                    TextFont {
                                                        font: font.clone(),
                                                        font_size: 16.0,
                                                        ..default()
                                                    },
                                                    TextColor(Color::srgb(0.8, 0.8, 0.8)),
                                                ));
                                                return;
                                            }

                                            for s in items {
                                                let name = s.id.display_name();
                                                let hint = if s.id == equipped.weapon {
                                                    "（已装备）"
                                                } else {
                                                    ""
                                                };

                                                let kind = db
                                                    .weapon(s.id)
                                                    .map(|w| format!("{:?}", w.kind))
                                                    .unwrap_or_else(|| "-".to_string());

                                                list.spawn((
                                                    Button,
                                                    EquipmentUiButton,
                                                    InventoryItemButton { item_id: s.id },
                                                    Node {
                                                        width: Val::Percent(100.0),
                                                        height: Val::Px(34.0),
                                                        flex_direction: FlexDirection::Row,
                                                        justify_content: JustifyContent::SpaceBetween,
                                                        align_items: AlignItems::Center,
                                                        padding: UiRect::horizontal(Val::Px(10.0)),
                                                        ..default()
                                                    },
                                                    BackgroundColor(Color::srgb(0.25, 0.25, 0.35)),
                                                    BorderRadius::all(Val::Px(6.0)),
                                                ))
                                                .with_children(|b| {
                                                    b.spawn((
                                                        Text::new(format!("{name} {hint}")),
                                                        TextFont {
                                                            font: font.clone(),
                                                            font_size: 16.0,
                                                            ..default()
                                                        },
                                                        TextColor(Color::WHITE),
                                                    ));
                                                    b.spawn((
                                                        Text::new(format!("x{}  |  {}", s.count, kind)),
                                                        TextFont {
                                                            font: font.clone(),
                                                            font_size: 14.0,
                                                            ..default()
                                                        },
                                                        TextColor(Color::srgb(0.9, 0.9, 0.3)),
                                                    ));
                                                });
                                            }
                                        });
                                });
                        });
                });
        });
}

/// 递归删除 UI 树
fn despawn_tree(entity: Entity, commands: &mut Commands, children_q: &Query<&Children>) {
    if let Ok(children) = children_q.get(entity) {
        for child in children.iter() {
            despawn_tree(child, commands, children_q);
        }
    }
    commands.entity(entity).despawn();
}
