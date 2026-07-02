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

use bevy::prelude::*;

use crate::{
    AppSystems, PausableSystems,
    collision::{
        CollisionMapKey, CollisionMasks, DynamicObstacleTiles, LiveObstacleItem, is_walkable,
    },
    components::{CurrentTilePosition, DesiredTilePosition, DynamicObstacle},
    demo::level::TILE_SIZE,
    screens::Screen,
    stats::EnclosureId,
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
#[derive(Component, Reflect, Default)]
#[reflect(Component, Default)]
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

fn update_desired_position(
    screen_state: Res<State<Screen>>,
    masks: Res<CollisionMasks>,
    persisted_obstacles: Query<(&EnclosureId, &DynamicObstacleTiles)>,
    live_obstacles: Query<LiveObstacleItem<'_>>,
    mut movement_query: Query<(
        Entity,
        &mut MovementController,
        &mut DesiredTilePosition,
        &CurrentTilePosition,
        Option<&DynamicObstacle>,
    )>,
) {
    let collision_key = CollisionMapKey::for_screen(screen_state.get());
    if !masks.contains(collision_key) {
        return;
    }

    for (entity, mut controller, mut desired, current, dynamic_obstacle) in &mut movement_query {
        if *desired != *current {
            controller.intent = None;
            continue;
        }
        if let Some(intent) = &controller.intent {
            let mut next_pos = current.0;
            match intent {
                MovementIntent::Up => next_pos.y = next_pos.y.saturating_add(1),
                MovementIntent::Down => next_pos.y = next_pos.y.saturating_sub(1),
                MovementIntent::Left => next_pos.x = next_pos.x.saturating_sub(1),
                MovementIntent::Right => next_pos.x = next_pos.x.saturating_add(1),
            }

            let ignore = dynamic_obstacle.is_some().then_some(entity);
            if is_walkable(
                &masks,
                &persisted_obstacles,
                &live_obstacles,
                collision_key,
                next_pos,
                ignore,
            ) {
                desired.0 = next_pos;
            }

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
    mut movement_query: Query<(
        &mut Transform,
        &mut MovementDuration,
        &DesiredTilePosition,
        &mut CurrentTilePosition,
    )>,
) {
    for (mut transform, mut duration, desired, mut current) in &mut movement_query {
        if *desired == *current {
            continue;
        }

        info!(
            "Applying movement from {:?} to {:?} with timer {:?} ",
            current.0, desired.0, duration.0
        );
        duration.0.tick(time.delta());

        // the translation should be tweened. When the tween exists, check if it's finished and remove this timer. This is just for demonstration purposes.
        if !duration.0.is_finished() {
            continue;
        }
        // For demonstration purposes, we simply snap the player to the desired position.
        // In a real game, you would want to move the player smoothly towards the desired position
        // based on the controller's max speed and the time delta.
        transform.translation.x = desired.0.x as f32 * TILE_SIZE as f32;
        transform.translation.y = desired.0.y as f32 * TILE_SIZE as f32;

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
