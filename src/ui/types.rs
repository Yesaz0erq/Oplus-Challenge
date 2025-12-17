// src/ui/types.rs
use bevy::prelude::*;

pub const RESOLUTIONS: &[(u32, u32)] = &[(1280, 720), (1600, 900), (1920, 1080)];

#[derive(Resource)]
pub struct GameSettings {
    pub resolution_index: usize,
    /// 0.0 ~ 1.0
    pub volume: f32,
    pub fullscreen: bool,
}

impl Default for GameSettings {
    fn default() -> Self {
        Self {
            resolution_index: 0,
            volume: 0.8,
            fullscreen: false,
        }
    }
}

#[derive(Resource, Default)]
pub struct SelectedSlot(pub Option<String>);
