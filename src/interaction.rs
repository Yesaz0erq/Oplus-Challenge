use std::time::Duration;
use bevy::prelude::*;
use crate::movement::Player;
use crate::state::GameState;

pub struct InteractionPlugin;

#[derive(Message)]
pub struct InteractEvent;

#[derive(Resource)]
struct InteractionFlash(Timer);

impl Plugin for InteractionPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<InteractEvent>()
            .insert_resource(InteractionFlash(Timer::new(
                Duration::from_millis(200),
                TimerMode::Once,
            )))
            .add_systems(Update, emit_interact_event.run_if(in_state(GameState::InGame)))
            .add_systems(Update, start_interaction_feedback)
            .add_systems(Update, apply_interaction_feedback);
    }
}

fn emit_interact_event(keyboard: Res<ButtonInput<KeyCode>>, mut writer: MessageWriter<InteractEvent>) {
    if keyboard.just_pressed(KeyCode::KeyE) {
        writer.write(InteractEvent);
    }
}

fn start_interaction_feedback(
    time: Res<Time>,
    mut flash: ResMut<InteractionFlash>,
    mut events: MessageReader<InteractEvent>,
    mut player_query: Query<&mut Transform, With<Player>>,
) {
    for _ in events.read() {
        info!("InteractEvent triggered");
        flash.0.reset();
        flash.0.tick(time.delta());
        for mut transform in &mut player_query {
            transform.scale = Vec3::splat(1.15);
        }
    }
}

fn apply_interaction_feedback(
    time: Res<Time>,
    mut flash: ResMut<InteractionFlash>,
    mut player_query: Query<&mut Transform, With<Player>>,
) {
    if flash.0.is_finished() {
        if flash.0.elapsed_secs() > 0.0 {
            for mut transform in &mut player_query {
                transform.scale = Vec3::ONE;
            }
        }
        return;
    }

    flash.0.tick(time.delta());
    if flash.0.is_finished() {
        for mut transform in &mut player_query {
            transform.scale = Vec3::ONE;
        }
    }
}