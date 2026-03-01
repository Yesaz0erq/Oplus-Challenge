use bevy::prelude::*;

pub const PANEL_TEXTURE: &str = "ui/panel_window.png";
pub const BUTTON_LARGE_TEXTURE: &str = "ui/button_large.png";
pub const BUTTON_SMALL_TEXTURE: &str = "ui/button_small.png";
pub const SLOT_TEXTURE: &str = "ui/slot_frame.png";
pub const HUD_SLOT_TEXTURE: &str = "ui/hud_slot.png";

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

pub fn overlay() -> Color {
    Color::srgba(0.01, 0.02, 0.06, 0.54)
}

pub fn panel_tint() -> Color {
    Color::srgba(0.10, 0.13, 0.24, 0.78)
}

pub fn subpanel_tint() -> Color {
    Color::srgba(0.08, 0.10, 0.20, 0.74)
}

pub fn inset_tint() -> Color {
    Color::srgba(0.05, 0.07, 0.15, 0.62)
}

pub fn tooltip_tint() -> Color {
    Color::srgba(0.04, 0.06, 0.13, 0.84)
}

pub fn button_idle() -> Color {
    Color::srgb(0.30, 0.35, 0.49)
}

pub fn button_hover() -> Color {
    Color::srgb(0.46, 0.54, 0.72)
}

pub fn button_pressed() -> Color {
    Color::srgb(0.80, 0.84, 0.94)
}

pub fn button_primary() -> Color {
    Color::srgb(0.40, 0.46, 0.62)
}

pub fn button_confirm() -> Color {
    Color::srgb(0.39, 0.46, 0.56)
}

pub fn button_danger() -> Color {
    Color::srgb(0.48, 0.30, 0.36)
}

pub fn selected_fill() -> Color {
    Color::srgb(0.53, 0.59, 0.76)
}

pub fn equipped_fill() -> Color {
    Color::srgb(0.74, 0.66, 0.46)
}

pub fn slot_fill() -> Color {
    Color::srgb(0.28, 0.32, 0.45)
}

pub fn slot_hover() -> Color {
    Color::srgb(0.58, 0.66, 0.84)
}

pub fn text_primary() -> Color {
    Color::srgb(0.94, 0.96, 1.0)
}

pub fn text_muted() -> Color {
    Color::srgb(0.76, 0.81, 0.91)
}

pub fn text_accent() -> Color {
    Color::srgb(0.86, 0.80, 0.62)
}
