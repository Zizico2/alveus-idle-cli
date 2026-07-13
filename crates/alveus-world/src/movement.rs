//! Tile-based character controller: translate movement intent into tile steps.
//!
//! - [`MovementController`] intent is set from keyboard input (in `alveus_headless`)
//!   or from `GameCommand::Move` over BRP.
//! - Movement is applied one tile at a time, snapping to the tile grid.

use bevy::prelude::*;

use alveus_app::{AppSystems, PausableSystems, Screen, tile_interaction_enabled};
use alveus_collision::{
    CollisionMapKey, CollisionMasks, DynamicObstacleTiles, LiveObstacleItem, is_walkable,
};
use alveus_components::{
    CurrentTilePosition, DesiredTilePosition, DynamicObstacle, MovementController,
    MovementDuration, MovementIntent, TILE_SIZE,
};
use alveus_types::EnclosureId;

pub(super) fn plugin(app: &mut App) {
    app.add_systems(
        Update,
        (update_desired_position, start_movement, apply_movement)
            .chain()
            .in_set(AppSystems::Update)
            .in_set(PausableSystems)
            .run_if(tile_interaction_enabled),
    );
}

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

        if !duration.0.is_finished() {
            continue;
        }
        transform.translation.x = desired.0.x as f32 * TILE_SIZE as f32;
        transform.translation.y = desired.0.y as f32 * TILE_SIZE as f32;

        current.0 = desired.0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alveus_app::Menu;
    use alveus_collision::CollisionMapKey;
    use alveus_components::{Player, TilePosition};
    use bevy::state::app::StatesPlugin;
    use bevy::time::TimeUpdateStrategy;
    use std::collections::HashSet;
    use std::time::Duration;

    #[test]
    fn care_picker_freezes_an_in_flight_player_tile_step() {
        let mut app = App::new();
        app.add_plugins(StatesPlugin);
        app.add_plugins(MinimalPlugins);
        app.add_plugins(alveus_app::plugin);
        app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs(1)));

        let mut masks = CollisionMasks::default();
        masks.set_static_mask(CollisionMapKey::Overview, HashSet::new());
        app.insert_resource(masks);
        app.add_plugins(plugin);
        app.world_mut()
            .resource_mut::<NextState<Screen>>()
            .set(Screen::Gameplay);
        app.world_mut()
            .resource_mut::<NextState<Menu>>()
            .set(Menu::CareItemPicker);
        app.update();

        let start = TilePosition { x: 4, y: 7 };
        let player = app
            .world_mut()
            .spawn((
                Player,
                CurrentTilePosition(start),
                DesiredTilePosition(TilePosition { x: 5, y: 7 }),
                MovementController {
                    intent: Some(MovementIntent::Right),
                },
                MovementDuration(Timer::from_seconds(0.1, TimerMode::Once)),
                Transform::default(),
            ))
            .id();

        for _ in 0..3 {
            app.update();
        }

        assert_eq!(
            app.world().get::<CurrentTilePosition>(player).unwrap().0,
            start
        );
        assert_eq!(
            app.world().get::<DesiredTilePosition>(player).unwrap().0,
            TilePosition { x: 5, y: 7 }
        );
    }
}
