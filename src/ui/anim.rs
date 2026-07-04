//! Lightweight UI motion toolkit: easing, tweens and hover motion for `bevy_ui`
//! nodes. Used across menus and the skill-card HUD to give the GUI a polished,
//! commercial feel without any new art assets.
//!
//! Some easing curves / builders are part of the reusable toolkit surface and
//! may not all be wired up yet.
#![allow(dead_code)]

use bevy::prelude::*;

/// Easing curves used by [`UiTween`] and ad-hoc animation systems.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Ease {
    Linear,
    #[default]
    OutCubic,
    /// Overshoots slightly past the target then settles — gives a satisfying
    /// "pop". Great for cards being drawn.
    OutBack,
    InOutCubic,
    /// Hermite smoothstep, gentle in and out.
    Smooth,
}

impl Ease {
    pub fn apply(self, t: f32) -> f32 {
        let t = t.clamp(0.0, 1.0);
        match self {
            Ease::Linear => t,
            Ease::OutCubic => 1.0 - (1.0 - t).powi(3),
            Ease::OutBack => {
                const C1: f32 = 1.70158;
                const C3: f32 = C1 + 1.0;
                1.0 + C3 * (t - 1.0).powi(3) + C1 * (t - 1.0).powi(2)
            }
            Ease::InOutCubic => {
                if t < 0.5 {
                    4.0 * t * t * t
                } else {
                    1.0 - (-2.0 * t + 2.0).powi(3) / 2.0
                }
            }
            Ease::Smooth => t * t * (3.0 - 2.0 * t),
        }
    }
}

/// What to do with the [`UiTween`] component once it finishes.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum OnComplete {
    /// Remove the `UiTween` component, leaving the node at its final transform.
    #[default]
    Remove,
    /// Keep the component (it will simply stay at `t == 1.0`).
    Hold,
}

/// A one-shot tween that animates a node's [`UiTransform`] (translation in px,
/// scale, rotation) and, optionally, its alpha. Insert it on any UI node; the
/// [`tick_ui_tweens`] system drives it.
#[derive(Component, Clone)]
pub struct UiTween {
    pub timer: Timer,
    pub ease: Ease,
    pub from_translation: Vec2,
    pub to_translation: Vec2,
    pub from_scale: Vec2,
    pub to_scale: Vec2,
    pub from_rotation: f32,
    pub to_rotation: f32,
    /// `(from, to)` alpha multiplier applied to `ImageNode`, `BackgroundColor`
    /// and `TextColor` where present. `None` leaves colors untouched.
    pub alpha: Option<(f32, f32)>,
    pub on_complete: OnComplete,
}

impl Default for UiTween {
    fn default() -> Self {
        Self {
            timer: Timer::from_seconds(0.3, TimerMode::Once),
            ease: Ease::OutCubic,
            from_translation: Vec2::ZERO,
            to_translation: Vec2::ZERO,
            from_scale: Vec2::ONE,
            to_scale: Vec2::ONE,
            from_rotation: 0.0,
            to_rotation: 0.0,
            alpha: None,
            on_complete: OnComplete::Remove,
        }
    }
}

impl UiTween {
    /// Convenience: a panel/popup "pop-in" — scale up from `scale0` and fade in.
    pub fn pop_in(secs: f32, scale0: f32) -> Self {
        Self {
            timer: Timer::from_seconds(secs, TimerMode::Once),
            ease: Ease::OutBack,
            from_scale: Vec2::splat(scale0),
            to_scale: Vec2::ONE,
            alpha: Some((0.0, 1.0)),
            ..default()
        }
    }

    /// Builder: set the translation tween (px offsets).
    pub fn with_translation(mut self, from: Vec2, to: Vec2) -> Self {
        self.from_translation = from;
        self.to_translation = to;
        self
    }

    /// Builder: set the rotation tween (radians, clockwise).
    pub fn with_rotation(mut self, from: f32, to: f32) -> Self {
        self.from_rotation = from;
        self.to_rotation = to;
        self
    }
}

fn val2_px(v: Vec2) -> Val2 {
    Val2::px(v.x, v.y)
}

/// Advances every [`UiTween`] and writes the interpolated transform / alpha.
pub fn tick_ui_tweens(
    mut commands: Commands,
    time: Res<Time>,
    mut q: Query<(
        Entity,
        &mut UiTween,
        &mut UiTransform,
        Option<&mut ImageNode>,
        Option<&mut BackgroundColor>,
        Option<&mut TextColor>,
    )>,
) {
    let dt = time.delta();
    for (entity, mut tween, mut transform, image, background, text) in &mut q {
        tween.timer.tick(dt);
        let t = tween.ease.apply(tween.timer.fraction());

        let translation = tween.from_translation.lerp(tween.to_translation, t);
        let scale = tween.from_scale.lerp(tween.to_scale, t);
        let rotation = tween.from_rotation + (tween.to_rotation - tween.from_rotation) * t;

        transform.translation = val2_px(translation);
        transform.scale = scale;
        transform.rotation = Rot2::radians(rotation);

        if let Some((a0, a1)) = tween.alpha {
            let a = a0 + (a1 - a0) * t;
            if let Some(mut image) = image {
                image.color = image.color.with_alpha(a);
            }
            if let Some(mut background) = background {
                background.0 = background.0.with_alpha(a);
            }
            if let Some(mut text) = text {
                text.0 = text.0.with_alpha(a);
            }
        }

        if tween.timer.is_finished() && tween.on_complete == OnComplete::Remove {
            commands.entity(entity).remove::<UiTween>();
        }
    }
}

/// Continuous hover feedback for buttons: smoothly lifts and scales the node
/// while hovered/pressed. Color swaps stay with each panel's own handler so we
/// don't fight over `BackgroundColor`. Requires a [`UiTransform`] on the node
/// (inserted automatically by `skin::spawn_text_button`).
#[derive(Component, Clone, Copy)]
pub struct HoverMotion {
    /// Upward lift in px applied on hover (positive = up).
    pub lift: f32,
    /// Scale multiplier on hover.
    pub hover_scale: f32,
    /// Extra "press" scale (usually slightly below 1.0 for a tactile dip).
    pub press_scale: f32,
    /// Exponential smoothing rate (higher = snappier).
    pub smoothing: f32,
}

impl Default for HoverMotion {
    fn default() -> Self {
        Self {
            lift: 4.0,
            hover_scale: 1.04,
            press_scale: 0.97,
            smoothing: 16.0,
        }
    }
}

pub fn apply_hover_motion(
    time: Res<Time>,
    mut q: Query<(&Interaction, &HoverMotion, &mut UiTransform)>,
) {
    let dt = time.delta_secs();
    for (interaction, motion, mut transform) in &mut q {
        let (target_y, target_scale) = match interaction {
            Interaction::Pressed => (-motion.lift * 0.5, motion.press_scale),
            Interaction::Hovered => (-motion.lift, motion.hover_scale),
            Interaction::None => (0.0, 1.0),
        };

        let alpha = 1.0 - (-motion.smoothing * dt).exp();

        let cur_y = match transform.translation.y {
            Val::Px(px) => px,
            _ => 0.0,
        };
        let new_y = cur_y + (target_y - cur_y) * alpha;
        transform.translation = Val2::px(0.0, new_y);

        let cur_scale = transform.scale.x;
        let new_scale = cur_scale + (target_scale - cur_scale) * alpha;
        transform.scale = Vec2::splat(new_scale);
    }
}

/// Registers the UI animation systems.
pub struct UiAnimPlugin;

impl Plugin for UiAnimPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (tick_ui_tweens, apply_hover_motion));
    }
}
