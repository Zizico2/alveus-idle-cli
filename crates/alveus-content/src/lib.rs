//! Interaction helpers and room-object ids.
//!
//! Static gameplay tables (items, animals, placements, economy knobs) live in
//! [`alveus_configs`] and are re-exported here so callers have a single content
//! entry point. Placement for room objects comes from Tiled maps.

use alveus_types::AnimalId;
use bevy::prelude::*;

pub use alveus_configs::{
    ANIMALS_DATA, AnimalPlacementDef, AnimalStaticData, ENCLOSURES_DATA, EnclosureStaticData,
    ItemStaticData, NUTRITION_HOUSE_ROOM, OFFLINE_WANDER_STEPS_PER_HOUR, OVERVIEW_PLAYER_SPAWN,
    POLLY_PLACEMENT, PUSH_POP_ENCLOSURE_ROOM, PUSH_POP_PLACEMENT, RoomTileConfig, STAT_FULL,
    STAT_SCALE, animal_data, animal_default_placement, enclosure_data, enclosure_for_animal,
    item_data, item_display_name,
};
pub use alveus_types::{ItemId, TileBounds, TilePosition};

// ---------------------------------------------------------
// Room objects & interactions (Tiled-authored object ids)
// ---------------------------------------------------------

/// Identifies an interactable room object. Authored on interior tiles via Tiled custom properties.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Hash, Reflect, Default)]
#[reflect(Component, Default)]
pub enum RoomObjectId {
    #[default]
    DietFridge,
    SeedChest,
    PushPopFeedingDish,
    CompostBin,
}

pub fn room_object_display_name(object_id: RoomObjectId) -> &'static str {
    match object_id {
        RoomObjectId::DietFridge => "Diet Fridge",
        RoomObjectId::SeedChest => "Seed Chest",
        RoomObjectId::PushPopFeedingDish => "Push Pop's Feeding Dish",
        RoomObjectId::CompostBin => "Compost Bin",
    }
}

pub fn default_tile_position(animal_id: AnimalId) -> Option<TilePosition> {
    animal_default_placement(animal_id).map(|placement| placement.home_position)
}

/// Returns true when the player tile is on or orthogonally adjacent to the object tile.
pub fn can_interact(player: TilePosition, object: TilePosition) -> bool {
    let dx = player.x.abs_diff(object.x);
    let dy = player.y.abs_diff(object.y);
    (dx == 0 && dy == 0) || (dx + dy == 1)
}

pub fn tile_in_bounds(tile: TilePosition, bounds: TileBounds) -> bool {
    tile.x >= bounds.bottom_left.x
        && tile.x <= bounds.top_right.x
        && tile.y >= bounds.bottom_left.y
        && tile.y <= bounds.top_right.y
}

pub fn adjacent_tiles(tile: TilePosition) -> [TilePosition; 4] {
    [
        TilePosition {
            x: tile.x.saturating_sub(1),
            y: tile.y,
        },
        TilePosition {
            x: tile.x + 1,
            y: tile.y,
        },
        TilePosition {
            x: tile.x,
            y: tile.y.saturating_sub(1),
        },
        TilePosition {
            x: tile.x,
            y: tile.y + 1,
        },
    ]
}
