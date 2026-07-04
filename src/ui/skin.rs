//! Shared visual design system: palette tokens, decoration bundles (rounded
//! corners + gradients + soft shadows) and a canonical button builder. Kept
//! code-only so the GUI reads as a polished commercial indie game with no new
//! art assets.
//!
//! Palette/decoration helpers form a reusable surface; not all are wired up yet.
#![allow(dead_code)]

use bevy::prelude::*;

use crate::i18n::Language;
use crate::ui::anim::HoverMotion;

pub const PANEL_TEXTURE: &str = "ui/panel_window.png";
pub const BUTTON_LARGE_TEXTURE: &str = "ui/button_large.png";
pub const BUTTON_SMALL_TEXTURE: &str = "ui/button_small.png";
pub const SLOT_TEXTURE: &str = "ui/slot_frame.png";
pub const HUD_SLOT_TEXTURE: &str = "ui/hud_slot.png";
pub const LATIN_FONT: &str = "fonts/ChillPixels-Matrix.otf";
pub const CJK_FONT: &str = "fonts/YuFanLixing.otf";

// ---------------------------------------------------------------------------
// Typography scale
// ---------------------------------------------------------------------------

pub const FONT_TITLE: f32 = 44.0;
pub const FONT_HEADING: f32 = 26.0;
pub const FONT_BUTTON: f32 = 24.0;
pub const FONT_BODY: f32 = 16.0;
pub const FONT_CAPTION: f32 = 12.0;

pub const BUTTON_HEIGHT: f32 = 52.0;
pub const PANEL_RADIUS: f32 = 16.0;
pub const BUTTON_RADIUS: f32 = 12.0;
pub const CARD_RADIUS: f32 = 10.0;

pub fn ui_font(asset_server: &AssetServer, lang: Language) -> Handle<Font> {
    if lang == Language::ZhCn {
        asset_server.load(CJK_FONT)
    } else {
        asset_server.load(LATIN_FONT)
    }
}

// ---------------------------------------------------------------------------
// Texture handles (kept for surfaces that still use the bitmap skin)
// ---------------------------------------------------------------------------

pub fn panel(asset_server: &AssetServer) -> Handle<Image> {
    asset_server.load(PANEL_TEXTURE)
}

pub fn button_large(asset_server: &AssetServer) -> Handle<Image> {
    asset_server.load(BUTTON_LARGE_TEXTURE)
}

pub fn button_small(asset_server: &AssetServer) -> Handle<Image> {
    asset_server.load(BUTTON_SMALL_TEXTURE)
}

pub fn slot(asset_server: &AssetServer) -> Handle<Image> {
    asset_server.load(SLOT_TEXTURE)
}

pub fn hud_slot(asset_server: &AssetServer) -> Handle<Image> {
    asset_server.load(HUD_SLOT_TEXTURE)
}

// ---------------------------------------------------------------------------
// Palette
// ---------------------------------------------------------------------------

pub fn overlay() -> Color {
    Color::srgba(0.01, 0.02, 0.05, 0.66)
}

pub fn panel_tint() -> Color {
    Color::srgba(0.09, 0.11, 0.19, 0.96)
}

pub fn panel_grad_top() -> Color {
    Color::srgba(0.13, 0.16, 0.27, 0.97)
}

pub fn panel_grad_bottom() -> Color {
    Color::srgba(0.05, 0.06, 0.12, 0.98)
}

pub fn subpanel_tint() -> Color {
    Color::srgba(0.08, 0.10, 0.18, 0.92)
}

pub fn inset_tint() -> Color {
    Color::srgba(0.05, 0.07, 0.14, 0.85)
}

pub fn tooltip_tint() -> Color {
    Color::srgba(0.04, 0.06, 0.13, 0.92)
}

pub fn border_soft() -> Color {
    Color::srgba(0.56, 0.64, 0.88, 0.30)
}

pub fn border_strong() -> Color {
    Color::srgba(0.78, 0.84, 1.0, 0.55)
}

pub fn shadow_color() -> Color {
    Color::srgba(0.0, 0.0, 0.0, 0.55)
}

pub fn accent_gold() -> Color {
    Color::srgb(0.88, 0.74, 0.42)
}

pub fn accent_glow() -> Color {
    Color::srgb(0.46, 0.80, 1.0)
}

// Button color families (idle / hover / pressed share the hover/pressed tints).
pub fn button_idle() -> Color {
    Color::srgb(0.22, 0.27, 0.40)
}

pub fn button_hover() -> Color {
    Color::srgb(0.34, 0.41, 0.59)
}

pub fn button_pressed() -> Color {
    Color::srgb(0.52, 0.60, 0.82)
}

pub fn button_primary() -> Color {
    Color::srgb(0.26, 0.40, 0.62)
}

pub fn button_confirm() -> Color {
    Color::srgb(0.24, 0.44, 0.40)
}

pub fn button_danger() -> Color {
    Color::srgb(0.50, 0.26, 0.32)
}

pub fn selected_fill() -> Color {
    Color::srgb(0.53, 0.59, 0.76)
}

pub fn equipped_fill() -> Color {
    Color::srgb(0.74, 0.66, 0.46)
}

pub fn slot_fill() -> Color {
    Color::srgb(0.20, 0.24, 0.36)
}

pub fn slot_hover() -> Color {
    Color::srgb(0.58, 0.66, 0.84)
}

pub fn text_primary() -> Color {
    Color::srgb(0.95, 0.97, 1.0)
}

pub fn text_muted() -> Color {
    Color::srgb(0.72, 0.78, 0.90)
}

pub fn text_dim() -> Color {
    Color::srgb(0.55, 0.61, 0.74)
}

pub fn text_accent() -> Color {
    Color::srgb(0.90, 0.80, 0.55)
}

// ---------------------------------------------------------------------------
// Decoration constructors
// ---------------------------------------------------------------------------

/// Vertical (top→bottom) linear gradient fill.
pub fn vgradient(top: Color, bottom: Color) -> BackgroundGradient {
    BackgroundGradient(vec![
        LinearGradient::to_bottom(vec![ColorStop::auto(top), ColorStop::auto(bottom)]).into(),
    ])
}

/// `BorderRadius` is a field of [`Node`] in Bevy 0.18 (not a standalone
/// component), so callers set `Node { border_radius: skin::radius(..), .. }`.
pub fn radius(px: f32) -> BorderRadius {
    BorderRadius::all(Val::Px(px))
}

/// Soft ambient drop shadow for floating panels.
pub fn shadow_soft() -> BoxShadow {
    BoxShadow::new(
        shadow_color(),
        Val::Px(0.0),
        Val::Px(12.0),
        Val::Px(2.0),
        Val::Px(32.0),
    )
}

/// Tighter shadow for small elements (cards, chips).
pub fn shadow_card() -> BoxShadow {
    BoxShadow::new(
        Color::srgba(0.0, 0.0, 0.0, 0.45),
        Val::Px(0.0),
        Val::Px(6.0),
        Val::Px(1.0),
        Val::Px(14.0),
    )
}

/// Window/panel chrome: gradient fill, soft border and drop shadow. Spread onto
/// a `Node` that sets `border: UiRect::all(..)` and
/// `border_radius: skin::radius(skin::PANEL_RADIUS)`.
pub fn panel_decoration() -> impl Bundle {
    (
        BackgroundColor(panel_tint()),
        vgradient(panel_grad_top(), panel_grad_bottom()),
        BorderColor::all(border_soft()),
        shadow_soft(),
    )
}

/// Lighter inset sub-panel (rows, list backgrounds). Pair with
/// `border_radius: skin::radius(10.0)` on the node.
pub fn subpanel_decoration() -> impl Bundle {
    (
        BackgroundColor(subpanel_tint()),
        BorderColor::all(border_soft()),
    )
}

/// Small rounded "pill" badge for HUD readouts. Pair with a large
/// `border_radius` on the node.
pub fn pill_decoration() -> impl Bundle {
    (
        BackgroundColor(inset_tint()),
        BorderColor::all(border_soft()),
        shadow_card(),
    )
}

/// HUD/offer card chrome. `tint` colors the gradient so different rarities or
/// states can be distinguished. Pair with `border_radius: skin::radius(CARD_RADIUS)`.
pub fn card_decoration(tint: Color) -> impl Bundle {
    let top = tint.with_alpha(0.92).lighter(0.06);
    let bottom = tint.with_alpha(0.96).darker(0.10);
    (
        BackgroundColor(tint),
        vgradient(top, bottom),
        BorderColor::all(border_strong()),
        shadow_card(),
    )
}

// ---------------------------------------------------------------------------
// Button builder
// ---------------------------------------------------------------------------

#[derive(Clone, Copy)]
pub enum ButtonKind {
    Neutral,
    Primary,
    Confirm,
    Danger,
}

impl ButtonKind {
    pub fn idle(self) -> Color {
        match self {
            ButtonKind::Neutral => button_idle(),
            ButtonKind::Primary => button_primary(),
            ButtonKind::Confirm => button_confirm(),
            ButtonKind::Danger => button_danger(),
        }
    }
}

/// Decoration for a text button. Color swaps stay with each panel's own
/// interaction handler (it owns `BackgroundColor`); we add the border and
/// shadow here. Pair with `border_radius: skin::radius(BUTTON_RADIUS)`.
pub fn button_decoration(kind: ButtonKind) -> impl Bundle {
    (
        BackgroundColor(kind.idle()),
        BorderColor::all(border_soft()),
        shadow_card(),
    )
}

/// Canonical full-width text button. Spawns `Button + Node + decoration +
/// UiTransform + HoverMotion + action`, with a centered label. Returns the
/// button entity. The caller supplies an `action` component (its own enum).
pub fn spawn_text_button(
    parent: &mut ChildSpawnerCommands<'_>,
    font: &Handle<Font>,
    label: impl Into<String>,
    kind: ButtonKind,
    action: impl Bundle,
) -> Entity {
    parent
        .spawn((
            Button,
            Node {
                width: Val::Percent(100.0),
                height: Val::Px(BUTTON_HEIGHT),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                border: UiRect::all(Val::Px(1.5)),
                border_radius: radius(BUTTON_RADIUS),
                ..default()
            },
            button_decoration(kind),
            UiTransform::IDENTITY,
            HoverMotion::default(),
            action,
        ))
        .with_children(|button| {
            button.spawn((
                Text::new(label.into()),
                TextFont {
                    font: font.clone().into(),
                    font_size: FontSize::from(FONT_BUTTON),
                    ..default()
                },
                TextColor(text_primary()),
            ));
        })
        .id()
}
