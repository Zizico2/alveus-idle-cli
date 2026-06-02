//! Handle player input and translate it into movement through a character
//! controller. A character controller is the collection of systems that govern
//! the movement of characters.
//!
//! In our case, the character controller has the following logic:
//! - Set [`MovementController`] intent based on directional keyboard input.
//!   This is done in the `player` module, as it is specific to the player
//!   character.
//! - Apply movement based on [`MovementController`] intent and maximum speed.
//! - Wrap the character within the window.
//!
//! Note that the implementation used here is limited for demonstration
//! purposes. If you want to move the player in a smoother way,
//! consider using a [fixed timestep](https://github.com/bevyengine/bevy/blob/main/examples/movement/physics_in_fixed_timestep.rs).

use bevy::{prelude::*, window::PrimaryWindow};

use crate::{
    AppSystems, PausableSystems,
    demo::player::{CurrentTilePosition, DesiredTilePosition},
};

pub(super) fn plugin(app: &mut App) {
    app.add_systems(
        Update,
        (
            update_desired_position,
            start_movement,
            apply_movement,
            // DISABLED
            // apply_screen_wrap
        )
            .chain()
            .in_set(AppSystems::Update)
            .in_set(PausableSystems),
    );
}

/// These are the movement parameters for our character controller.
/// For now, this is only used for a single player, but it could power NPCs or
/// other players as well.
#[derive(Component, Reflect)]
#[reflect(Component)]
pub struct MovementController {
    /// The direction the character wants to move in.
    pub intent: Option<MovementIntent>,
    // Maximum speed in world units per second.
    // 1 world unit = 1 pixel when using the default 2D camera and no physics engine.
    // pub max_speed: f32,
}

#[derive(Reflect, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum MovementIntent {
    Up,
    Down,
    Left,
    Right,
}

// this should match the animation duration for a single tile step
#[derive(Component)]
pub struct MovementDuration(pub Timer);

impl Default for MovementController {
    fn default() -> Self {
        Self {
            intent: None,
            // max_speed: 400.0,
        }
    }
}

fn update_desired_position(
    time: Res<Time>,
    mut movement_query: Query<(
        &mut MovementController,
        &mut DesiredTilePosition,
        &CurrentTilePosition,
    )>,
) {
    for (mut controller, mut desired, current) in &mut movement_query {
        if *desired != *current {
            // If the player is already trying to move, we don't want to change their desired position.
            // reset the intent since the movement was rejected
            controller.intent = None;
            continue;
        }
        if let Some(intent) = &controller.intent {
            match intent {
                MovementIntent::Up => desired.0.y = desired.0.y.saturating_sub(1),
                MovementIntent::Down => desired.0.y = desired.0.y.saturating_add(1),
                MovementIntent::Left => desired.0.x = desired.0.x.saturating_sub(1),
                MovementIntent::Right => desired.0.x = desired.0.x.saturating_add(1),
            }
            // reset the intent since we've processed it
            controller.intent = None;
        }
    }
}

fn start_movement(
    mut movement_query: Query<
        (
            &mut MovementDuration,
            &DesiredTilePosition,
            &CurrentTilePosition,
        ),
        Changed<DesiredTilePosition>,
    >,
) {
    for (mut duration, desired, current) in &mut movement_query {
        info!("Starting movement from {:?} to {:?}", current.0, desired.0);
        if *desired == *current {
            continue;
        }
        duration.0.reset();
        duration.0.unpause();
    }
}

fn apply_movement(
    time: Res<Time>,
    mut movement_query: Query<
        (
            &mut Transform,
            &mut MovementDuration,
            &DesiredTilePosition,
            &mut CurrentTilePosition,
        ),
        // Changed<DesiredTilePosition>,
    >,
) {
    for (mut transform, mut duration, desired, mut current) in &mut movement_query {
        info!(
            "Applying movement from {:?} to {:?} with timer {:?} ",
            current.0, desired.0, duration.0
        );
        duration.0.tick(time.delta());
        if *desired == *current {
            continue;
        }

        // the translation should be tweened. When the tween exists, check if it's finished and remove this timer. This is just for demonstration purposes.
        if !duration.0.is_finished() {
            continue;
        }
        // For demonstration purposes, we simply snap the player to the desired position.
        // In a real game, you would want to move the player smoothly towards the desired position
        // based on the controller's max speed and the time delta.
        transform.translation.x = desired.0.x as f32 * 32.0; // Assuming 32 pixels per tile
        transform.translation.y = desired.0.y as f32 * 32.0;

        current.0 = desired.0;
    }
}

// DISABLED
// #[derive(Component, Reflect)]
// #[reflect(Component)]
// pub struct ScreenWrap;

// fn apply_screen_wrap(
//     window: Single<&Window, With<PrimaryWindow>>,
//     mut wrap_query: Query<&mut Transform, With<ScreenWrap>>,
// ) {
//     let size = window.size() + 256.0;
//     let half_size = size / 2.0;
//     for mut transform in &mut wrap_query {
//         let position = transform.translation.xy();
//         let wrapped = (position + half_size).rem_euclid(size) - half_size;
//         transform.translation = wrapped.extend(transform.translation.z);
//     }
// }
