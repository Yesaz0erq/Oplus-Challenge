use bevy::image::Image;
use bevy::prelude::*;
use bevy::ui::{
    AlignItems, Display, FlexDirection, GridAutoFlow, JustifyContent, PositionType,
    RepeatedGridTrack, UiRect,
};
use std::collections::HashMap;

use crate::inventory::{Inventory, ItemStack};
use crate::movement::Player;
use crate::state::GameState;

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

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum WeaponKind {
    Melee,
    Ranged,
}

#[derive(Component, Clone)]
pub struct EquipmentSet {
    pub weapon_kind: WeaponKind,
    pub weapon_damage: f32,
    pub weapon_attack_cooldown: f32,
    pub weapon_projectile_speed: f32,
    pub weapon_projectile_lifetime: f32,
    pub melee_range: f32,
    pub melee_width: f32,
}

impl Default for EquipmentSet {
    fn default() -> Self {
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

    pub fn icon_path(self) -> &'static str {
        match self {
            ItemId::RustySword => "items/rusty_sword.png",
            ItemId::MagicWand => "items/magic_wand.png",
            ItemId::HunterBow => "items/hunter_bow.png",
        }
    }
}

#[derive(Clone)]
pub struct WeaponDef {
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

        weapons.insert(
            ItemId::RustySword,
            WeaponDef {
                kind: WeaponKind::Melee,
                damage: 20.0,
                cooldown: 0.6,
                projectile_speed: 400.0,
                projectile_lifetime: 1.0,
                melee_range: 80.0,
                melee_width: 40.0,
            },
        );

        weapons.insert(
            ItemId::MagicWand,
            WeaponDef {
                kind: WeaponKind::Ranged,
                damage: 14.0,
                cooldown: 0.35,
                projectile_speed: 520.0,
                projectile_lifetime: 1.2,
                melee_range: 60.0,
                melee_width: 30.0,
            },
        );

        weapons.insert(
            ItemId::HunterBow,
            WeaponDef {
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
    pub fn from_weapon(def: &WeaponDef) -> Self {
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

#[derive(Component)]
pub struct EquipmentUiRoot;

#[derive(Component)]
struct EquipmentSlotButton;

#[derive(Component)]
struct InventoryItemButton {
    pub item_id: ItemId,
}

#[derive(Component)]
struct CloseButton;

#[derive(Message, Clone, Copy, Debug)]
struct EquipWeaponMsg {
    item_id: ItemId,
}

#[derive(Resource, Default)]
struct EquipmentUiDirty(pub bool);

#[derive(Resource, Default)]
struct HoveredItem(pub Option<ItemId>);

#[derive(Component)]
struct PlayerAttrText;

#[derive(Component)]
struct WeaponDataText;

#[derive(Component)]
struct ItemDetailText;

pub struct EquipmentPlugin;

impl Plugin for EquipmentPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<EquipmentUiConfig>()
            .init_resource::<ItemDatabase>()
            .init_resource::<EquipmentUiDirty>()
            .init_resource::<HoveredItem>()
            .add_message::<EquipWeaponMsg>()
            .add_systems(
                Update,
                ensure_player_inventory_and_equipment.run_if(in_state(GameState::InGame)),
            )
            .add_systems(Update, toggle_equipment_ui.run_if(in_state(GameState::InGame)))
            .add_systems(Update, handle_slot_buttons.run_if(in_state(GameState::InGame)))
            .add_systems(Update, handle_close_button.run_if(in_state(GameState::InGame)))
            .add_systems(
                Update,
                apply_equip_weapon_messages.run_if(in_state(GameState::InGame)),
            )
            .add_systems(
                Update,
                rebuild_equipment_ui_when_dirty.run_if(in_state(GameState::InGame)),
            )
            .add_systems(
                Update,
                (update_hovered_item, update_detail_panel).run_if(in_state(GameState::InGame)),
            );
    }
}

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
            let mut inv = Inventory::new(120);
            inv.try_add(ItemId::MagicWand, 1, 99);
            inv.try_add(ItemId::HunterBow, 1, 99);
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

fn toggle_equipment_ui(
    keyboard: Res<ButtonInput<KeyCode>>,
    cfg: Res<EquipmentUiConfig>,
    mut commands: Commands,
    ui_root_q: Query<Entity, With<EquipmentUiRoot>>,
    asset_server: Res<AssetServer>,
    db: Res<ItemDatabase>,
    player_q: Query<(&EquipmentSet, &EquippedItems, &Inventory), With<Player>>,
    mut dirty: ResMut<EquipmentUiDirty>,
) {
    if !keyboard.just_pressed(cfg.toggle_key) {
        return;
    }

    if let Ok(root) = ui_root_q.single() {
        commands.entity(root).try_despawn();
        return;
    }

    let Ok((equip, equipped, inv)) = player_q.single() else {
        return;
    };

    dirty.0 = false;
    spawn_player_info_ui(&mut commands, &asset_server, &db, equip, equipped, inv);
}

fn spawn_player_info_ui(
    commands: &mut Commands,
    asset_server: &AssetServer,
    _db: &ItemDatabase,
    equip: &EquipmentSet,
    equipped: &EquippedItems,
    inv: &Inventory,
) {
    let font: Handle<Font> = asset_server.load("fonts/YuFanLixing.otf");
    let portrait: Handle<Image> = asset_server.load("character.png");

    let root = commands
        .spawn((
            EquipmentUiRoot,
            GlobalZIndex(100),
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(0.0),
                top: Val::Px(0.0),
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                padding: UiRect::all(Val::Px(12.0)),
                ..default()
            },
            BackgroundColor(Color::srgba(0.06, 0.06, 0.08, 0.95)),
        ))
        .id();

    commands.entity(root).with_children(|ui| {
        ui.spawn((
            Node {
                width: Val::Percent(92.0),
                height: Val::Percent(90.0),
                flex_direction: FlexDirection::Row,
                column_gap: Val::Px(12.0),
                ..default()
            },
            BackgroundColor(Color::srgba(0.04, 0.04, 0.06, 0.2)),
        ))
        .with_children(|panel| {
            panel
                .spawn((
                    Node {
                        width: Val::Px(320.0),
                        height: Val::Percent(100.0),
                        flex_direction: FlexDirection::Column,
                        align_items: AlignItems::Center,
                        padding: UiRect::all(Val::Px(10.0)),
                        ..default()
                    },
                    BackgroundColor(Color::srgba(0.08, 0.08, 0.10, 0.9)),
                ))
                .with_children(|left| {
                    left.spawn((
                        ImageNode {
                            image: portrait.clone(),
                            ..default()
                        },
                        Node {
                            width: Val::Px(280.0),
                            height: Val::Px(420.0),
                            margin: UiRect::all(Val::Px(8.0)),
                            ..default()
                        },
                    ));
                });

            panel
                .spawn((
                    Node {
                        width: Val::Percent(100.0),
                        height: Val::Percent(100.0),
                        flex_direction: FlexDirection::Column,
                        row_gap: Val::Px(10.0),
                        padding: UiRect::all(Val::Px(10.0)),
                        ..default()
                    },
                    BackgroundColor(Color::srgba(0.07, 0.07, 0.09, 0.9)),
                ))
                .with_children(|mid| {
                    mid.spawn((
                        Text::new("Inventory"),
                        TextFont {
                            font: font.clone(),
                            font_size: 22.0,
                            ..default()
                        },
                        TextColor(Color::WHITE),
                    ));

                    let cols: u16 = 10;
                    let rows: u16 = ((inv.slot_count() + cols as usize - 1) / cols as usize) as u16;
                    let cell = 36.0;

                    mid.spawn((
                        Node {
                            width: Val::Percent(100.0),
                            height: Val::Px(520.0),
                            display: Display::Grid,
                            grid_auto_flow: GridAutoFlow::Row,
                            grid_template_columns: RepeatedGridTrack::px(cols, cell),
                            grid_template_rows: RepeatedGridTrack::px(rows.max(1), cell),
                            row_gap: Val::Px(6.0),
                            column_gap: Val::Px(6.0),
                            ..default()
                        },
                        BackgroundColor(Color::srgba(0.02, 0.02, 0.03, 0.2)),
                    ))
                    .with_children(|grid| {
                        for idx in 0..inv.slot_count() {
                            let maybe = inv.slots[idx];
                            match maybe {
                                Some(ItemStack { id, .. }) => {
                                    grid.spawn((
                                        Button,
                                        EquipmentSlotButton,
                                        InventoryItemButton { item_id: id },
                                        Node {
                                            width: Val::Px(cell),
                                            height: Val::Px(cell),
                                            justify_content: JustifyContent::Center,
                                            align_items: AlignItems::Center,
                                            ..default()
                                        },
                                        BackgroundColor(Color::srgb(0.25, 0.25, 0.35)),
                                    ))
                                    .with_children(|btn| {
                                        let icon: Handle<Image> = asset_server.load(id.icon_path());
                                        btn.spawn((
                                            ImageNode { image: icon, ..default() },
                                            Node {
                                                width: Val::Px(32.0),
                                                height: Val::Px(32.0),
                                                ..default()
                                            },
                                        ));
                                    });
                                }
                                None => {
                                    grid.spawn((
                                        Node {
                                            width: Val::Px(cell),
                                            height: Val::Px(cell),
                                            ..default()
                                        },
                                        BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.15)),
                                    ));
                                }
                            }
                        }
                    });
                });

            panel
                .spawn((
                    Node {
                        width: Val::Px(380.0),
                        height: Val::Percent(100.0),
                        flex_direction: FlexDirection::Column,
                        row_gap: Val::Px(10.0),
                        padding: UiRect::all(Val::Px(10.0)),
                        ..default()
                    },
                    BackgroundColor(Color::srgba(0.06, 0.06, 0.08, 0.95)),
                ))
                .with_children(|right| {
                    right.spawn((
                        Text::new("Player"),
                        TextFont {
                            font: font.clone(),
                            font_size: 20.0,
                            ..default()
                        },
                        TextColor(Color::WHITE),
                    ));

                    right.spawn((
                        PlayerAttrText,
                        Text::new("HP: --/--   ATK: --"),
                        TextFont {
                            font: font.clone(),
                            font_size: 16.0,
                            ..default()
                        },
                        TextColor(Color::WHITE),
                    ));

                    right.spawn((
                        WeaponDataText,
                        Text::new(format!(
                            "Weapon: {}\nDMG: {:.0}\nCD: {:.2}\nRange: {:.0}",
                            equipped.weapon.display_name(),
                            equip.weapon_damage,
                            equip.weapon_attack_cooldown,
                            equip.melee_range
                        )),
                        TextFont {
                            font: font.clone(),
                            font_size: 16.0,
                            ..default()
                        },
                        TextColor(Color::WHITE),
                    ));

                    right.spawn((
                        Text::new("Item Details"),
                        TextFont {
                            font: font.clone(),
                            font_size: 20.0,
                            ..default()
                        },
                        TextColor(Color::WHITE),
                    ));

                    right.spawn((
                        ItemDetailText,
                        Text::new("Hover an item to see details."),
                        TextFont {
                            font: font.clone(),
                            font_size: 16.0,
                            ..default()
                        },
                        TextColor(Color::WHITE),
                    ));

                    right.spawn((
                        Button,
                        CloseButton,
                        Node {
                            width: Val::Percent(100.0),
                            height: Val::Px(44.0),
                            justify_content: JustifyContent::Center,
                            align_items: AlignItems::Center,
                            ..default()
                        },
                        BackgroundColor(Color::srgb(0.20, 0.20, 0.28)),
                    ))
                    .with_children(|b| {
                        b.spawn((
                            Text::new("Close"),
                            TextFont {
                                font: font.clone(),
                                font_size: 16.0,
                                ..default()
                            },
                            TextColor(Color::WHITE),
                        ));
                    });
                });
        });
    });
}

fn handle_slot_buttons(
    mut interactions: Query<
        (&Interaction, &mut BackgroundColor, Option<&InventoryItemButton>),
        (Changed<Interaction>, With<Button>, With<EquipmentSlotButton>),
    >,
    mut writer: MessageWriter<EquipWeaponMsg>,
) {
    for (interaction, mut bg, item_btn) in &mut interactions {
        match *interaction {
            Interaction::Pressed => {
                bg.0 = Color::srgb(0.8, 0.8, 1.0);
                if let Some(btn) = item_btn {
                    writer.write(EquipWeaponMsg { item_id: btn.item_id });
                }
            }
            Interaction::Hovered => {
                bg.0 = Color::srgb(0.6, 0.6, 0.8);
            }
            Interaction::None => {
                bg.0 = Color::srgb(0.25, 0.25, 0.35);
            }
        }
    }
}

fn handle_close_button(
    mut commands: Commands,
    root_q: Query<Entity, With<EquipmentUiRoot>>,
    mut q: Query<(&Interaction, &mut BackgroundColor), (Changed<Interaction>, With<Button>, With<CloseButton>)>,
) {
    for (interaction, mut bg) in &mut q {
        match *interaction {
            Interaction::Pressed => {
                bg.0 = Color::srgb(0.35, 0.35, 0.45);
                if let Ok(root) = root_q.single() {
                    commands.entity(root).try_despawn();
                }
            }
            Interaction::Hovered => bg.0 = Color::srgb(0.28, 0.28, 0.40),
            Interaction::None => bg.0 = Color::srgb(0.20, 0.20, 0.28),
        }
    }
}

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
        if new_id == equipped.weapon {
            continue;
        }

        if inv.try_remove_one(new_id) {
            let old = equipped.weapon;
            inv.try_add(old, 1, 99);
            equipped.weapon = new_id;
            if let Some(def) = db.weapon(new_id) {
                *equip_set = EquipmentSet::from_weapon(def);
            }
            dirty.0 = true;
        }
    }
}

fn rebuild_equipment_ui_when_dirty(
    dirty: Res<EquipmentUiDirty>,
    ui_root_q: Query<Entity, With<EquipmentUiRoot>>,
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    db: Res<ItemDatabase>,
    player_q: Query<(&EquipmentSet, &EquippedItems, &Inventory), With<Player>>,
) {
    if !dirty.is_changed() || !dirty.0 {
        return;
    }

    let Ok((equip, equipped, inv)) = player_q.single() else {
        return;
    };

    if let Ok(root) = ui_root_q.single() {
        commands.entity(root).try_despawn();
    }

    spawn_player_info_ui(&mut commands, &asset_server, &db, equip, equipped, inv);
}

fn update_hovered_item(
    mut hovered: ResMut<HoveredItem>,
    q: Query<(&Interaction, &InventoryItemButton), With<Button>>,
) {
    let mut found = None;
    for (interaction, btn) in &q {
        if *interaction == Interaction::Hovered {
            found = Some(btn.item_id);
            break;
        }
    }
    hovered.0 = found;
}

fn update_detail_panel(
    hovered: Res<HoveredItem>,
    db: Res<ItemDatabase>,
    mut texts: ParamSet<(
        Query<&mut Text, With<ItemDetailText>>,
        Query<&mut Text, With<PlayerAttrText>>,
        Query<&mut Text, With<WeaponDataText>>,
    )>,
    hp_q: Query<&crate::health::Health, With<Player>>,
    equip_q: Query<&EquipmentSet, With<Player>>,
    equipped_q: Query<&EquippedItems, With<Player>>,
) {
    {
        let mut item_q = texts.p0();
        if let Ok(mut t) = item_q.single_mut() {
            if let Some(item_id) = hovered.0 {
                let mut s = String::new();
                s.push_str(item_id.display_name());
                s.push_str("\n\n");
                if let Some(w) = db.weapon(item_id) {
                    s.push_str(&format!(
                        "Type: Weapon\nKind: {:?}\nDMG: {:.0}\nCD: {:.2}\nProjSpd: {:.0}\nProjLife: {:.2}\nMeleeRange: {:.0}\nMeleeWidth: {:.0}",
                        w.kind,
                        w.damage,
                        w.cooldown,
                        w.projectile_speed,
                        w.projectile_lifetime,
                        w.melee_range,
                        w.melee_width
                    ));
                } else {
                    s.push_str("No detailed data.");
                }
                t.0 = s;
            } else {
                t.0 = "Hover an item to see details.".to_string();
            }
        }
    }

    {
        let mut attr_q = texts.p1();
        if let Ok(mut t) = attr_q.single_mut() {
            if let (Ok(hp), Ok(equip)) = (hp_q.single(), equip_q.single()) {
                t.0 = format!("HP: {:.0}/{:.0}   ATK: {:.0}", hp.current, hp.max, equip.weapon_damage);
            }
        }
    }

    {
        let mut weapon_q = texts.p2();
        if let Ok(mut t) = weapon_q.single_mut() {
            if let (Ok(equip), Ok(eq)) = (equip_q.single(), equipped_q.single()) {
                t.0 = format!(
                    "Weapon: {}\nDMG: {:.0}\nCD: {:.2}\nRange: {:.0}",
                    eq.weapon.display_name(),
                    equip.weapon_damage,
                    equip.weapon_attack_cooldown,
                    equip.melee_range
                );
            }
        }
    }
}