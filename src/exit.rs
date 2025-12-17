use bevy::app::AppExit;
use bevy::prelude::*;

pub struct ExitPlugin;

impl Plugin for ExitPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<AppExit>();
    }
}