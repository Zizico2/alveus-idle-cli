//! Tiled-driven interior map hooks: tile interactables from custom properties.

use bevy::prelude::*;
use bevy_ecs_tiled::prelude::*;
use crate::components::TilePosition;

pub(super) fn plugin(app: &mut App) {
    app.add_observer(bridge_interior_tile_position);
}

fn interior_tile_entity(event: &On<TiledEvent<TileCreated>>) -> Option<(Entity, TilePosition)> {
    let pos = event.get_tile_pos()?;
    let entity = event.get_tile_entity().unwrap_or(event.entity);
    Some((
        entity,
        TilePosition {
            x: pos.x,
            y: pos.y,
        },
    ))
}

/// Adds [`TilePosition`] to every interior tile entity when the map spawns it.
fn bridge_interior_tile_position(
    event: On<TiledEvent<TileCreated>>,
    mut commands: Commands,
) {
    let Some((entity, tile_pos)) = interior_tile_entity(&event) else {
        return;
    };
    commands.entity(entity).insert(tile_pos);
}
