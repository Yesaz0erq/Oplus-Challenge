use bevy::image::Image;
use bevy::prelude::*;
use bevy::ui::{
    AlignItems, Display, FlexDirection, GridAutoFlow, JustifyContent, PositionType,
    RepeatedGridTrack, UiRect,
};
use bevy::window::PrimaryWindow;
use std::collections::HashMap;

use crate::i18n::{L10n, Language};
use crate::inventory::{INVENTORY_PAGE_SLOT_COUNT, Inventory, ItemStack};
use crate::movement::Player;
use crate::skills::SkillBagUiRoot;
use crate::state::GameState;
use crate::ui::EscBlockingUi;
use crate::ui::pause_menu::SuppressPauseMenuOnce;
use crate::ui::skin;
use crate::ui::types::GameSettings;

const INVENTORY_DEFAULT_PAGES: usize = 2;
const INVENTORY_PAGE_COLS: u16 = 6;
const INVENTORY_PAGE_ROWS: u16 = 4;
const INVENTORY_CELL_SIZE: f32 = 104.0;
const INVENTORY_CELL_GAP: f32 = 8.0;

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

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Default)]
pub enum ItemId {
    #[default]
    RustySword,
    MagicWand,
    HunterBow,
}

impl ItemId {
    pub const fn to_u32(self) -> u32 {
        match self {
            ItemId::RustySword => 1,
            ItemId::MagicWand => 2,
            ItemId::HunterBow => 3,
        }
    }

    pub const fn from_u32(v: u32) -> Option<Self> {
        match v {
            1 => Some(ItemId::RustySword),
            2 => Some(ItemId::MagicWand),
            3 => Some(ItemId::HunterBow),
            _ => None,
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

#[derive(Component, Default)]
pub struct EquippedItems {
    pub weapon: ItemId,
}

#[derive(Component, Clone, Debug)]
pub struct PlayerMemory {
    pub level: u32,
    pub skill_capacity: usize,
}

impl Default for PlayerMemory {
    fn default() -> Self {
        Self {
            level: 1,
            skill_capacity: 3,
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
    pub count: u32,
    pub slot_index: usize,
    pub is_equipped: bool,
}

#[derive(Component)]
struct CloseButton;

#[derive(Component)]
struct InventoryPageButton {
    delta: i32,
}

#[derive(Component)]
struct InventoryPageText;

#[derive(Component)]
struct HoverCompareTooltip;

#[derive(Component)]
struct MemoryInfoButton;

#[derive(Message, Clone, Copy, Debug)]
struct EquipWeaponMsg {
    item_id: ItemId,
}

#[derive(Resource, Default)]
struct EquipmentUiDirty(pub bool);

#[derive(Resource, Default)]
struct HoveredItem(pub Option<HoveredInventoryItem>);

#[derive(Clone, Copy, Debug)]
struct HoveredInventoryItem {
    item_id: ItemId,
    count: u32,
    slot_index: usize,
    is_equipped: bool,
}

#[derive(Resource, Default)]
struct InventoryUiPageState {
    current_page: usize,
}

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
            .init_resource::<InventoryUiPageState>()
            .add_message::<EquipWeaponMsg>()
            .add_systems(
                Update,
                ensure_player_inventory_and_equipment.run_if(in_game_or_paused),
            )
            .add_systems(Update, toggle_equipment_ui.run_if(in_game_or_paused))
            .add_systems(
                Update,
                close_equipment_ui_on_esc
                    .run_if(in_game_or_paused)
                    .after(crate::input::EscInputSet),
            )
            .add_systems(Update, handle_slot_buttons.run_if(in_game_or_paused))
            .add_systems(
                Update,
                handle_inventory_page_buttons.run_if(in_game_or_paused),
            )
            .add_systems(Update, handle_close_button.run_if(in_game_or_paused))
            .add_systems(
                Update,
                apply_equip_weapon_messages.run_if(in_game_or_paused),
            )
            .add_systems(
                Update,
                rebuild_equipment_ui_when_dirty.run_if(in_game_or_paused),
            )
            .add_systems(
                Update,
                (
                    update_hovered_item,
                    update_detail_panel,
                    update_hover_compare_tooltip,
                )
                    .run_if(in_game_or_paused),
            );
    }
}

fn in_game_or_paused(state: Res<State<GameState>>) -> bool {
    matches!(state.get(), GameState::InGame | GameState::Paused)
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
            Option<&PlayerMemory>,
        ),
        With<Player>,
    >,
) {
    for (e, inv, equipped, equip_set, memory) in &q {
        if inv.is_none() {
            let mut inv = Inventory::new(INVENTORY_PAGE_SLOT_COUNT * INVENTORY_DEFAULT_PAGES);
            inv.try_add(ItemId::MagicWand, 1, 99);
            inv.try_add(ItemId::HunterBow, 1, 99);
            commands.entity(e).insert(inv);
        }

        let weapon_id = equipped.map(|x| x.weapon).unwrap_or_default();

        if equipped.is_none() {
            commands
                .entity(e)
                .insert(EquippedItems { weapon: weapon_id });
        }

        if equip_set.is_none() {
            if let Some(def) = db.weapon(weapon_id) {
                commands.entity(e).insert(EquipmentSet::from_weapon(def));
            } else {
                commands.entity(e).insert(EquipmentSet::default());
            }
        }

        if memory.is_none() {
            commands.entity(e).insert(PlayerMemory::default());
        }
    }
}

fn toggle_equipment_ui(
    keyboard: Res<ButtonInput<KeyCode>>,
    cfg: Res<EquipmentUiConfig>,
    mut commands: Commands,
    ui_root_q: Query<Entity, With<EquipmentUiRoot>>,
    skill_bag_q: Query<Entity, With<SkillBagUiRoot>>,
    asset_server: Res<AssetServer>,
    db: Res<ItemDatabase>,
    player_q: Query<(&EquipmentSet, &EquippedItems, &Inventory, &PlayerMemory), With<Player>>,
    mut dirty: ResMut<EquipmentUiDirty>,
    mut page_state: ResMut<InventoryUiPageState>,
    settings: Res<GameSettings>,
    current_state: Res<State<GameState>>,
    mut next_state: ResMut<NextState<GameState>>,
    mut suppress_pause_menu_once: ResMut<SuppressPauseMenuOnce>,
) {
    if !keyboard.just_pressed(cfg.toggle_key) {
        return;
    }

    if let Ok(root) = ui_root_q.single() {
        commands.entity(root).try_despawn();
        if matches!(current_state.get(), GameState::Paused) {
            suppress_pause_menu_once.0 = false;
            next_state.set(GameState::InGame);
        }
        return;
    }

    if !skill_bag_q.is_empty() {
        for root in &skill_bag_q {
            commands.entity(root).try_despawn();
        }
        if matches!(current_state.get(), GameState::Paused) {
            suppress_pause_menu_once.0 = false;
            next_state.set(GameState::InGame);
        }
        return;
    }

    if matches!(current_state.get(), GameState::Paused) {
        return;
    }

    let Ok((equip, equipped, inv, memory)) = player_q.single() else {
        return;
    };

    dirty.0 = false;
    page_state.current_page = 0;
    spawn_player_info_ui(
        &mut commands,
        &asset_server,
        &db,
        equip,
        equipped,
        inv,
        memory,
        &mut page_state,
        settings.language,
    );
    suppress_pause_menu_once.0 = true;
    next_state.set(GameState::Paused);
}

fn close_equipment_ui_on_esc(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut commands: Commands,
    ui_root_q: Query<Entity, With<EquipmentUiRoot>>,
    current_state: Res<State<GameState>>,
    mut next_state: ResMut<NextState<GameState>>,
    mut suppress_pause_menu_once: ResMut<SuppressPauseMenuOnce>,
) {
    if !keyboard.just_pressed(KeyCode::Escape) {
        return;
    }

    if let Ok(root) = ui_root_q.single() {
        commands.entity(root).try_despawn();
        suppress_pause_menu_once.0 = false;
        match current_state.get() {
            GameState::InGame | GameState::Paused => next_state.set(GameState::InGame),
            _ => {}
        }
    }
}

fn spawn_player_info_ui(
    commands: &mut Commands,
    asset_server: &AssetServer,
    _db: &ItemDatabase,
    equip: &EquipmentSet,
    equipped: &EquippedItems,
    inv: &Inventory,
    memory: &PlayerMemory,
    page_state: &mut InventoryUiPageState,
    lang: Language,
) {
    let font = skin::ui_font(asset_server, lang);
    let portrait: Handle<Image> = asset_server.load("character.png");
    let slots_per_page = INVENTORY_PAGE_SLOT_COUNT;
    let total_pages = inv
        .slot_count()
        .max(slots_per_page)
        .div_ceil(slots_per_page);
    page_state.current_page = page_state.current_page.min(total_pages.saturating_sub(1));
    let current_page = page_state.current_page;
    let page_start = current_page * slots_per_page;
    let page_end = page_start + slots_per_page;
    let grid_h = INVENTORY_PAGE_ROWS as f32 * INVENTORY_CELL_SIZE
        + (INVENTORY_PAGE_ROWS.saturating_sub(1) as f32) * INVENTORY_CELL_GAP;

    let root = commands
        .spawn((
            EquipmentUiRoot,
            EscBlockingUi,
            GlobalZIndex(100),
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(0.0),
                top: Val::Px(0.0),
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                padding: UiRect::all(Val::Px(10.0)),
                ..default()
            },
            BackgroundColor(skin::overlay()),
        ))
        .id();

    commands.entity(root).with_children(|ui| {
        ui.spawn((
            Node {
                width: Val::Percent(78.0),
                max_width: Val::Px(980.0),
                height: Val::Percent(72.0),
                max_height: Val::Px(620.0),
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Stretch,
                column_gap: Val::Px(14.0),
                padding: UiRect::all(Val::Px(14.0)),
                border: UiRect::all(Val::Px(1.5)),
                border_radius: skin::radius(skin::PANEL_RADIUS),
                ..default()
            },
            skin::panel_decoration(),
            UiTransform::IDENTITY,
            crate::ui::anim::UiTween::pop_in(0.26, 0.96),
        ))
        .with_children(|panel| {
            panel
                .spawn((
                    Node {
                        width: Val::Px(250.0),
                        height: Val::Auto,
                        align_self: AlignSelf::Stretch,
                        flex_direction: FlexDirection::Column,
                        justify_content: JustifyContent::SpaceBetween,
                        align_items: AlignItems::Stretch,
                        padding: UiRect::all(Val::Px(10.0)),
                        row_gap: Val::Px(8.0),
                        overflow: Overflow::clip(),
                        border: UiRect::all(Val::Px(1.0)),
                        border_radius: skin::radius(10.0),
                        ..default()
                    },
                    BackgroundColor(skin::subpanel_tint()),
                    BorderColor::all(skin::border_soft()),
                ))
                .with_children(|left| {
                    left.spawn((Node {
                        width: Val::Percent(100.0),
                        flex_grow: 1.0,
                        flex_direction: FlexDirection::Column,
                        align_items: AlignItems::Center,
                        justify_content: JustifyContent::FlexStart,
                        row_gap: Val::Px(8.0),
                        ..default()
                    },))
                        .with_children(|top| {
                            top.spawn((
                                ImageNode {
                                    image: portrait.clone(),
                                    ..default()
                                },
                                Node {
                                    width: Val::Px(150.0),
                                    height: Val::Px(220.0),
                                    margin: UiRect::all(Val::Px(8.0)),
                                    ..default()
                                },
                            ));

                            top.spawn((Node {
                                width: Val::Percent(100.0),
                                padding: UiRect::axes(Val::Px(6.0), Val::Px(2.0)),
                                justify_content: JustifyContent::Center,
                                ..default()
                            },))
                                .with_children(|text_wrap| {
                                    text_wrap.spawn((
                                        Text::new(format!(
                                            "{}\n{}",
                                            L10n::equipment_equipped_weapon_header(lang),
                                            L10n::item_name(lang, equipped.weapon)
                                        )),
                                        TextLayout::justify(Justify::Center),
                                        TextFont {
                                            font: font.clone().into(),
                                            font_size: FontSize::from(16.0),
                                            ..default()
                                        },
                                        TextColor(skin::text_accent()),
                                    ));
                                });

                            top.spawn((Node {
                                width: Val::Percent(100.0),
                                padding: UiRect::horizontal(Val::Px(8.0)),
                                ..default()
                            },))
                                .with_children(|stats| {
                                    stats.spawn((
                                        PlayerAttrText,
                                        Text::new(L10n::hp_atk(lang, 0.0, 0.0, 0.0)),
                                        TextFont {
                                            font: font.clone().into(),
                                            font_size: FontSize::from(12.0),
                                            ..default()
                                        },
                                        TextColor(skin::text_primary()),
                                    ));
                                });

                            top.spawn((Node {
                                width: Val::Percent(100.0),
                                padding: UiRect::horizontal(Val::Px(8.0)),
                                ..default()
                            },))
                                .with_children(|weapon| {
                                    weapon.spawn((
                                        WeaponDataText,
                                        Text::new(L10n::equipment_weapon_summary(
                                            lang,
                                            L10n::item_name(lang, equipped.weapon),
                                            equip.weapon_damage,
                                            equip.weapon_attack_cooldown,
                                            equip.melee_range,
                                        )),
                                        TextFont {
                                            font: font.clone().into(),
                                            font_size: FontSize::from(11.0),
                                            ..default()
                                        },
                                        TextColor(skin::text_primary()),
                                    ));
                                });

                            top.spawn((
                                Button,
                                MemoryInfoButton,
                                Node {
                                    width: Val::Percent(100.0),
                                    min_height: Val::Px(60.0),
                                    padding: UiRect::all(Val::Px(10.0)),
                                    margin: UiRect::top(Val::Px(4.0)),
                                    justify_content: JustifyContent::FlexStart,
                                    align_items: AlignItems::FlexStart,
                                    border: UiRect::all(Val::Px(1.5)),
                                    border_radius: skin::radius(skin::BUTTON_RADIUS),
                                    ..default()
                                },
                                BackgroundColor(skin::button_primary()),
                                BorderColor::all(skin::border_soft()),
                                skin::shadow_card(),
                                UiTransform::IDENTITY,
                                crate::ui::anim::HoverMotion::default(),
                            ))
                            .with_children(|mem| {
                                mem.spawn((
                                    Text::new(L10n::memory_summary(
                                        lang,
                                        L10n::memory_base_name(lang),
                                        memory.level,
                                        memory.skill_capacity,
                                    )),
                                    TextFont {
                                        font: font.clone().into(),
                                        font_size: FontSize::from(12.0),
                                        ..default()
                                    },
                                    TextColor(skin::text_muted()),
                                ));
                            });
                        });

                    left.spawn((
                        Button,
                        CloseButton,
                        Node {
                            width: Val::Percent(100.0),
                            height: Val::Px(42.0),
                            justify_content: JustifyContent::Center,
                            align_items: AlignItems::Center,
                            margin: UiRect::top(Val::Px(6.0)),
                            border: UiRect::all(Val::Px(1.5)),
                            border_radius: skin::radius(skin::BUTTON_RADIUS),
                            ..default()
                        },
                        BackgroundColor(skin::button_idle()),
                        BorderColor::all(skin::border_soft()),
                        skin::shadow_card(),
                        UiTransform::IDENTITY,
                        crate::ui::anim::HoverMotion::default(),
                    ))
                    .with_children(|b| {
                        b.spawn((
                            Text::new(L10n::close(lang)),
                            TextFont {
                                font: font.clone().into(),
                                font_size: FontSize::from(14.0),
                                ..default()
                            },
                            TextColor(skin::text_primary()),
                        ));
                    });
                });

            panel
                .spawn((
                    Node {
                        width: Val::Auto,
                        flex_grow: 1.0,
                        height: Val::Auto,
                        align_self: AlignSelf::Stretch,
                        flex_direction: FlexDirection::Column,
                        row_gap: Val::Px(8.0),
                        padding: UiRect::axes(Val::Px(14.0), Val::Px(12.0)),
                        overflow: Overflow::clip(),
                        border: UiRect::all(Val::Px(1.0)),
                        border_radius: skin::radius(10.0),
                        ..default()
                    },
                    BackgroundColor(skin::subpanel_tint()),
                    BorderColor::all(skin::border_soft()),
                ))
                .with_children(|mid| {
                    mid.spawn((Node {
                        width: Val::Percent(100.0),
                        min_height: Val::Px(50.0),
                        justify_content: JustifyContent::SpaceBetween,
                        align_items: AlignItems::Center,
                        padding: UiRect::top(Val::Px(6.0)),
                        ..default()
                    },))
                        .with_children(|header| {
                            header
                                .spawn((Node {
                                    width: Val::Px(260.0),
                                    justify_content: JustifyContent::FlexStart,
                                    align_items: AlignItems::Center,
                                    ..default()
                                },))
                                .with_children(|_| {});

                            header
                                .spawn((Node {
                                    flex_grow: 1.0,
                                    min_height: Val::Px(34.0),
                                    justify_content: JustifyContent::Center,
                                    align_items: AlignItems::Center,
                                    overflow: Overflow::clip(),
                                    padding: UiRect::top(Val::Px(2.0)),
                                    ..default()
                                },))
                                .with_children(|title| {
                                    title.spawn((
                                        Text::new(L10n::equipment_inventory(lang)),
                                        TextLayout::justify(Justify::Center),
                                        TextFont {
                                            font: font.clone().into(),
                                            font_size: FontSize::from(16.0),
                                            ..default()
                                        },
                                        TextColor(skin::text_primary()),
                                    ));
                                });

                            header
                                .spawn((Node {
                                    width: Val::Px(260.0),
                                    flex_direction: FlexDirection::Row,
                                    column_gap: Val::Px(8.0),
                                    align_items: AlignItems::Center,
                                    justify_content: JustifyContent::FlexEnd,
                                    ..default()
                                },))
                                .with_children(|pager| {
                                    spawn_small_control_button(
                                        pager,
                                        asset_server,
                                        &font,
                                        L10n::equipment_prev_page(lang),
                                        InventoryPageButton { delta: -1 },
                                    );
                                    pager.spawn((
                                        InventoryPageText,
                                        Text::new(L10n::equipment_page_label(
                                            lang,
                                            current_page + 1,
                                            total_pages,
                                            inv.slot_count(),
                                        )),
                                        TextFont {
                                            font: font.clone().into(),
                                            font_size: FontSize::from(15.0),
                                            ..default()
                                        },
                                        TextColor(skin::text_muted()),
                                    ));
                                    spawn_small_control_button(
                                        pager,
                                        asset_server,
                                        &font,
                                        L10n::equipment_next_page(lang),
                                        InventoryPageButton { delta: 1 },
                                    );
                                });
                        });

                    mid.spawn((
                        Node {
                            width: Val::Percent(100.0),
                            flex_grow: 1.0,
                            min_height: Val::Px(grid_h + 14.0),
                            display: Display::Grid,
                            grid_auto_flow: GridAutoFlow::Row,
                            grid_template_columns: RepeatedGridTrack::px(
                                INVENTORY_PAGE_COLS,
                                INVENTORY_CELL_SIZE,
                            ),
                            grid_template_rows: RepeatedGridTrack::px(
                                INVENTORY_PAGE_ROWS,
                                INVENTORY_CELL_SIZE,
                            ),
                            row_gap: Val::Px(INVENTORY_CELL_GAP),
                            column_gap: Val::Px(INVENTORY_CELL_GAP),
                            padding: UiRect::all(Val::Px(6.0)),
                            overflow: Overflow::clip(),
                            border_radius: skin::radius(8.0),
                            ..default()
                        },
                        BackgroundColor(skin::inset_tint()),
                    ))
                    .with_children(|grid| {
                        for idx in page_start..page_end {
                            let maybe = inv.slots.get(idx).copied().flatten();
                            match maybe {
                                Some(ItemStack { id, count }) => {
                                    let is_equipped = id == equipped.weapon;
                                    grid.spawn((
                                        Button,
                                        EquipmentSlotButton,
                                        InventoryItemButton {
                                            item_id: id,
                                            count,
                                            slot_index: idx,
                                            is_equipped,
                                        },
                                        Node {
                                            width: Val::Px(INVENTORY_CELL_SIZE),
                                            height: Val::Px(INVENTORY_CELL_SIZE),
                                            justify_content: JustifyContent::Center,
                                            align_items: AlignItems::Center,
                                            ..default()
                                        },
                                        BackgroundColor(if is_equipped {
                                            skin::equipped_fill()
                                        } else {
                                            skin::slot_fill()
                                        }),
                                        ImageNode::new(skin::slot(asset_server)),
                                    ))
                                    .with_children(|btn| {
                                        let icon: Handle<Image> = asset_server.load(id.icon_path());
                                        btn.spawn((
                                            ImageNode {
                                                image: icon,
                                                ..default()
                                            },
                                            Node {
                                                width: Val::Px(INVENTORY_CELL_SIZE - 14.0),
                                                height: Val::Px(INVENTORY_CELL_SIZE - 14.0),
                                                ..default()
                                            },
                                        ));

                                        if is_equipped {
                                            btn.spawn((
                                                Text::new(if lang == Language::ZhCn {
                                                    "装"
                                                } else {
                                                    "E"
                                                }),
                                                TextFont {
                                                    font: font.clone().into(),
                                                    font_size: FontSize::from(16.0),
                                                    ..default()
                                                },
                                                TextColor(skin::text_accent()),
                                                Node {
                                                    position_type: PositionType::Absolute,
                                                    right: Val::Px(3.0),
                                                    top: Val::Px(1.0),
                                                    ..default()
                                                },
                                            ));
                                        }

                                        if count > 1 {
                                            btn.spawn((
                                                Text::new(count.to_string()),
                                                TextFont {
                                                    font: font.clone().into(),
                                                    font_size: FontSize::from(14.0),
                                                    ..default()
                                                },
                                                TextColor(Color::WHITE),
                                                Node {
                                                    position_type: PositionType::Absolute,
                                                    right: Val::Px(3.0),
                                                    bottom: Val::Px(1.0),
                                                    ..default()
                                                },
                                            ));
                                        }
                                    });
                                }
                                None => {
                                    grid.spawn((
                                        Node {
                                            width: Val::Px(INVENTORY_CELL_SIZE),
                                            height: Val::Px(INVENTORY_CELL_SIZE),
                                            ..default()
                                        },
                                        BackgroundColor(skin::slot_fill()),
                                        ImageNode::new(skin::slot(asset_server)),
                                    ));
                                }
                            }
                        }
                    });
                });
        });

        ui.spawn((
            HoverCompareTooltip,
            GlobalZIndex(200),
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(0.0),
                top: Val::Px(0.0),
                width: Val::Px(360.0),
                max_width: Val::Px(360.0),
                padding: UiRect::all(Val::Px(14.0)),
                border: UiRect::all(Val::Px(1.5)),
                border_radius: skin::radius(12.0),
                overflow: Overflow::clip(),
                display: Display::None,
                ..default()
            },
            BackgroundColor(skin::tooltip_tint()),
            BorderColor::all(skin::border_soft()),
            skin::shadow_card(),
            Text::new(""),
            TextLayout::justify(Justify::Center),
            TextFont {
                font: font.clone().into(),
                font_size: FontSize::from(12.0),
                ..default()
            },
            TextColor(skin::text_primary()),
        ));
    });
}

fn handle_slot_buttons(
    mut interactions: Query<
        (
            &Interaction,
            &mut BackgroundColor,
            Option<&InventoryItemButton>,
        ),
        (
            Changed<Interaction>,
            With<Button>,
            With<EquipmentSlotButton>,
        ),
    >,
    mut writer: MessageWriter<EquipWeaponMsg>,
) {
    for (interaction, mut bg, item_btn) in &mut interactions {
        let base = match item_btn {
            Some(btn) if btn.is_equipped => skin::equipped_fill(),
            _ => skin::slot_fill(),
        };
        match *interaction {
            Interaction::Pressed => {
                bg.0 = skin::button_pressed();
                if let Some(btn) = item_btn {
                    writer.write(EquipWeaponMsg {
                        item_id: btn.item_id,
                    });
                }
            }
            Interaction::Hovered => {
                bg.0 = skin::slot_hover();
            }
            Interaction::None => {
                bg.0 = base;
            }
        }
    }
}

fn handle_inventory_page_buttons(
    mut interactions: Query<
        (&Interaction, &mut BackgroundColor, &InventoryPageButton),
        (Changed<Interaction>, With<Button>),
    >,
    mut page_state: ResMut<InventoryUiPageState>,
    mut dirty: ResMut<EquipmentUiDirty>,
    player_q: Query<&Inventory, With<Player>>,
) {
    let Ok(inv) = player_q.single() else {
        return;
    };
    let total_pages = inv
        .slot_count()
        .max(INVENTORY_PAGE_SLOT_COUNT)
        .div_ceil(INVENTORY_PAGE_SLOT_COUNT);

    for (interaction, mut bg, page_btn) in &mut interactions {
        match *interaction {
            Interaction::Pressed => {
                bg.0 = skin::button_pressed();
                let old = page_state.current_page;
                let max_page = total_pages.saturating_sub(1);
                let next = if page_btn.delta < 0 {
                    old.saturating_sub(page_btn.delta.unsigned_abs() as usize)
                } else {
                    old.saturating_add(page_btn.delta as usize).min(max_page)
                };
                if next != old {
                    page_state.current_page = next;
                    dirty.0 = true;
                }
            }
            Interaction::Hovered => bg.0 = skin::button_hover(),
            Interaction::None => bg.0 = skin::button_idle(),
        }
    }
}

fn handle_close_button(
    mut commands: Commands,
    root_q: Query<Entity, With<EquipmentUiRoot>>,
    current_state: Res<State<GameState>>,
    mut next_state: ResMut<NextState<GameState>>,
    mut suppress_pause_menu_once: ResMut<SuppressPauseMenuOnce>,
    mut q: Query<
        (&Interaction, &mut BackgroundColor),
        (Changed<Interaction>, With<Button>, With<CloseButton>),
    >,
) {
    for (interaction, mut bg) in &mut q {
        match *interaction {
            Interaction::Pressed => {
                bg.0 = skin::button_pressed();
                if let Ok(root) = root_q.single() {
                    commands.entity(root).try_despawn();
                    if matches!(current_state.get(), GameState::Paused) {
                        suppress_pause_menu_once.0 = false;
                        next_state.set(GameState::InGame);
                    }
                }
            }
            Interaction::Hovered => bg.0 = skin::button_hover(),
            Interaction::None => bg.0 = skin::button_idle(),
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
    player_q: Query<(&EquipmentSet, &EquippedItems, &Inventory, &PlayerMemory), With<Player>>,
    mut page_state: ResMut<InventoryUiPageState>,
    settings: Res<GameSettings>,
) {
    if !dirty.is_changed() || !dirty.0 {
        return;
    }

    let Ok((equip, equipped, inv, memory)) = player_q.single() else {
        return;
    };

    if let Ok(root) = ui_root_q.single() {
        commands.entity(root).try_despawn();
    }

    spawn_player_info_ui(
        &mut commands,
        &asset_server,
        &db,
        equip,
        equipped,
        inv,
        memory,
        &mut page_state,
        settings.language,
    );
}

fn update_hovered_item(
    mut hovered: ResMut<HoveredItem>,
    q: Query<(&Interaction, &InventoryItemButton), With<Button>>,
) {
    let mut found = None;
    for (interaction, btn) in &q {
        if *interaction == Interaction::Hovered {
            found = Some(HoveredInventoryItem {
                item_id: btn.item_id,
                count: btn.count,
                slot_index: btn.slot_index,
                is_equipped: btn.is_equipped,
            });
            break;
        }
    }
    hovered.0 = found;
}

fn update_detail_panel(
    hovered: Res<HoveredItem>,
    db: Res<ItemDatabase>,
    settings: Res<GameSettings>,
    mut texts: ParamSet<(
        Query<&mut Text, With<ItemDetailText>>,
        Query<&mut Text, With<PlayerAttrText>>,
        Query<&mut Text, With<WeaponDataText>>,
    )>,
    hp_q: Query<&crate::health::Health, With<Player>>,
    equip_q: Query<&EquipmentSet, With<Player>>,
    equipped_q: Query<&EquippedItems, With<Player>>,
) {
    let lang = settings.language;
    {
        let mut item_q = texts.p0();
        if let Ok(mut t) = item_q.single_mut() {
            if let Some(hover) = hovered.0 {
                let item_id = hover.item_id;
                let mut s = String::new();
                s.push_str(L10n::item_name(lang, item_id));
                if hover.is_equipped {
                    s.push_str(&format!("  ({})", L10n::equipped_tag(lang)));
                }
                if hover.count > 1 {
                    s.push_str(&format!("  x{}", hover.count));
                }
                s.push_str("\n\n");
                if let Some(w) = db.weapon(item_id) {
                    s.push_str(&format_weapon_block(
                        L10n::equipment_item_details(lang),
                        w,
                        lang,
                    ));
                } else {
                    s.push_str(if lang == Language::ZhCn {
                        "无详细数据。"
                    } else {
                        "No detailed data."
                    });
                }
                t.0 = s;
            } else {
                t.0 = L10n::equipment_hover_compare_hint(lang).to_string();
            }
        }
    }

    {
        let mut attr_q = texts.p1();
        if let Ok(mut t) = attr_q.single_mut()
            && let (Ok(hp), Ok(equip)) = (hp_q.single(), equip_q.single())
        {
            t.0 = L10n::hp_atk(lang, hp.current, hp.max, equip.weapon_damage);
        }
    }

    {
        let mut weapon_q = texts.p2();
        if let Ok(mut t) = weapon_q.single_mut()
            && let (Ok(equip), Ok(eq)) = (equip_q.single(), equipped_q.single())
        {
            t.0 = L10n::equipment_weapon_summary(
                lang,
                L10n::item_name(lang, eq.weapon),
                equip.weapon_damage,
                equip.weapon_attack_cooldown,
                equip.melee_range,
            );
        }
    }
}

fn format_weapon_block(title: &str, w: &WeaponDef, lang: Language) -> String {
    let dmg = if lang == Language::ZhCn {
        "伤害"
    } else {
        "DMG"
    };
    let cd = if lang == Language::ZhCn {
        "冷却"
    } else {
        "CD"
    };
    let proj_spd = if lang == Language::ZhCn {
        "弹速"
    } else {
        "ProjSpd"
    };
    let proj_life = if lang == Language::ZhCn {
        "弹体时长"
    } else {
        "ProjLife"
    };
    let range = if lang == Language::ZhCn {
        "射程"
    } else {
        "Range"
    };
    let width = if lang == Language::ZhCn {
        "宽度"
    } else {
        "Width"
    };
    format!(
        "{title}\n{}: {}\n{}: {:.0}  {}: {:.2}\n{}: {:.0}  {}: {:.2}\n{}: {:.0}  {}: {:.0}\n",
        if lang == Language::ZhCn {
            "类型"
        } else {
            "Kind"
        },
        weapon_kind_label(w.kind, lang),
        dmg,
        w.damage,
        cd,
        w.cooldown,
        proj_spd,
        w.projectile_speed,
        proj_life,
        w.projectile_lifetime,
        range,
        w.melee_range,
        width,
        w.melee_width
    )
}

fn format_weapon_stats_line(w: &WeaponDef, lang: Language) -> String {
    let dmg = if lang == Language::ZhCn {
        "伤害"
    } else {
        "DMG"
    };
    let cd = if lang == Language::ZhCn {
        "冷却"
    } else {
        "CD"
    };
    let proj_spd = if lang == Language::ZhCn {
        "弹速"
    } else {
        "ProjSpd"
    };
    let proj_life = if lang == Language::ZhCn {
        "弹体时长"
    } else {
        "ProjLife"
    };
    let range = if lang == Language::ZhCn {
        "射程"
    } else {
        "Range"
    };
    let width = if lang == Language::ZhCn {
        "宽度"
    } else {
        "Width"
    };
    format!(
        "{}: {}\n{}: {:.0}  {}: {:.2}\n{}: {:.0}  {}: {:.2}\n{}: {:.0}  {}: {:.0}",
        if lang == Language::ZhCn {
            "类型"
        } else {
            "Kind"
        },
        weapon_kind_label(w.kind, lang),
        dmg,
        w.damage,
        cd,
        w.cooldown,
        proj_spd,
        w.projectile_speed,
        proj_life,
        w.projectile_lifetime,
        range,
        w.melee_range,
        width,
        w.melee_width
    )
}

fn spawn_small_control_button(
    parent: &mut ChildSpawnerCommands<'_>,
    _asset_server: &AssetServer,
    font: &Handle<Font>,
    label: &str,
    page_btn: InventoryPageButton,
) {
    parent
        .spawn((
            Button,
            page_btn,
            Node {
                width: Val::Px(84.0),
                height: Val::Px(32.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                border: UiRect::all(Val::Px(1.5)),
                border_radius: skin::radius(skin::BUTTON_RADIUS),
                ..default()
            },
            BackgroundColor(skin::button_idle()),
            BorderColor::all(skin::border_soft()),
            UiTransform::IDENTITY,
            crate::ui::anim::HoverMotion::default(),
        ))
        .with_children(|btn| {
            btn.spawn((
                Text::new(label),
                TextFont {
                    font: font.clone().into(),
                    font_size: FontSize::from(13.0),
                    ..default()
                },
                TextColor(skin::text_primary()),
            ));
        });
}

fn update_hover_compare_tooltip(
    hovered: Res<HoveredItem>,
    db: Res<ItemDatabase>,
    settings: Res<GameSettings>,
    window_q: Query<&Window, With<PrimaryWindow>>,
    equipped_q: Query<&EquippedItems, With<Player>>,
    memory_q: Query<&PlayerMemory, With<Player>>,
    memory_btn_q: Query<&Interaction, (With<Button>, With<MemoryInfoButton>)>,
    mut tooltip_q: Query<(&mut Node, &mut Text), With<HoverCompareTooltip>>,
) {
    let Ok((mut node, mut text)) = tooltip_q.single_mut() else {
        return;
    };

    let Ok(window) = window_q.single() else {
        node.display = Display::None;
        return;
    };

    let Some(cursor) = window.cursor_position() else {
        node.display = Display::None;
        return;
    };

    let Ok(equipped) = equipped_q.single() else {
        node.display = Display::None;
        return;
    };

    if hovered.0.is_none() {
        let is_memory_hovered = memory_btn_q
            .iter()
            .any(|interaction| *interaction == Interaction::Hovered);
        if !is_memory_hovered {
            node.display = Display::None;
            return;
        }

        let Ok(memory) = memory_q.single() else {
            node.display = Display::None;
            return;
        };

        let lang = settings.language;
        node.display = Display::Flex;
        let tooltip_w = 360.0;
        let tooltip_h = 140.0;
        let x = (cursor.x + 18.0).min(window.width().max(tooltip_w) - tooltip_w);
        let y = (cursor.y + 18.0).min(window.height().max(tooltip_h) - tooltip_h);
        node.left = Val::Px(x.max(8.0));
        node.top = Val::Px(y.max(8.0));

        *text = Text::new(L10n::memory_summary(
            lang,
            L10n::memory_base_name(lang),
            memory.level,
            memory.skill_capacity,
        ));
        return;
    }

    let Some(hover) = hovered.0 else {
        node.display = Display::None;
        return;
    };

    node.display = Display::Flex;
    let tooltip_w = 360.0;
    let tooltip_h = 260.0;
    let x = (cursor.x + 18.0).min(window.width().max(tooltip_w) - tooltip_w);
    let y = (cursor.y + 18.0).min(window.height().max(tooltip_h) - tooltip_h);
    node.left = Val::Px(x.max(8.0));
    node.top = Val::Px(y.max(8.0));

    let lang = settings.language;
    let hover_name = L10n::item_name(lang, hover.item_id);
    let equipped_name = L10n::item_name(lang, equipped.weapon);
    let mut s = String::new();
    s.push_str(&format!(
        "{}: {}{}{}\n{}: {}",
        if lang == Language::ZhCn {
            "选中物品"
        } else {
            "Selected Item"
        },
        hover_name,
        if hover.is_equipped {
            format!(" ({})", L10n::equipped_tag(lang))
        } else {
            String::new()
        },
        if hover.count > 1 {
            format!("  x{}", hover.count)
        } else {
            String::new()
        },
        if lang == Language::ZhCn {
            "槽位"
        } else {
            "Slot"
        },
        hover.slot_index + 1
    ));
    s.push_str("\n\n");

    if let Some(w) = db.weapon(hover.item_id) {
        s.push_str(&format_weapon_block(
            if lang == Language::ZhCn {
                "选中物品属性"
            } else {
                "Selected Item Stats"
            },
            w,
            lang,
        ));
    } else {
        s.push_str(if lang == Language::ZhCn {
            "选中物品属性\n无详细数据\n"
        } else {
            "Selected Item Stats\nNo detailed data.\n"
        });
    }

    s.push('\n');
    if let Some(w_eq) = db.weapon(equipped.weapon) {
        s.push_str(&format!(
            "{}: {} ({})\n",
            if lang == Language::ZhCn {
                "当前装备"
            } else {
                "Equipped"
            },
            equipped_name,
            L10n::equipped_tag(lang)
        ));
        s.push_str(&format_weapon_stats_line(w_eq, lang));

        if let Some(w_hover) = db.weapon(hover.item_id) {
            s.push('\n');
            s.push_str(&format!(
                "{}: {} {:+.0} | {} {:+.2} | {} {:+.0}",
                if lang == Language::ZhCn {
                    "对比"
                } else {
                    "Compare"
                },
                if lang == Language::ZhCn {
                    "伤害"
                } else {
                    "DMG"
                },
                w_hover.damage - w_eq.damage,
                if lang == Language::ZhCn {
                    "冷却"
                } else {
                    "CD"
                },
                w_hover.cooldown - w_eq.cooldown,
                if lang == Language::ZhCn {
                    "射程"
                } else {
                    "Range"
                },
                w_hover.melee_range - w_eq.melee_range
            ));
        }
    }

    *text = Text::new(s);
}

fn weapon_kind_label(kind: WeaponKind, lang: Language) -> &'static str {
    match kind {
        WeaponKind::Melee => L10n::kind_melee(lang),
        WeaponKind::Ranged => L10n::kind_ranged(lang),
    }
}
