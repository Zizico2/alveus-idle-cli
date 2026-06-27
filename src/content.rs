//! Hardcoded game content synced with `design/data/*.json` and `design/rooms/*.json`.
//! Placement for room objects and animals comes from Tiled maps; interaction rules live here.

use bevy::prelude::*;
use crate::components::TilePosition;
use crate::stats::{AnimalId, EnclosureId};

// ---------------------------------------------------------
// Items (sync with design/data/items.json)
// ---------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Reflect)]
pub enum ItemId {
    TortoiseLeafyGreens,
    ChickenGrains,
}

impl ItemId {
    pub fn as_str(&self) -> &'static str {
        match self {
            ItemId::TortoiseLeafyGreens => "tortoise_leafy_greens",
            ItemId::ChickenGrains => "chicken_grains",
        }
    }
}

pub struct ItemStaticData {
    pub item_id: ItemId,
    pub display_name: &'static str,
}

pub const ITEMS_DATA: &[ItemStaticData] = &[
    ItemStaticData {
        item_id: ItemId::TortoiseLeafyGreens,
        display_name: "Tortoise Leafy Greens",
    },
    ItemStaticData {
        item_id: ItemId::ChickenGrains,
        display_name: "Chicken Grains",
    },
];

pub fn item_display_name(item_id: ItemId) -> &'static str {
    ITEMS_DATA
        .iter()
        .find(|i| i.item_id == item_id)
        .map(|i| i.display_name)
        .unwrap_or("Unknown Item")
}

// ---------------------------------------------------------
// Room objects & interactions (sync with design/rooms/*.json)
// ---------------------------------------------------------

/// Identifies an interactable room object. Authored on interior tiles via Tiled custom properties.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Hash, Reflect, Default)]
#[reflect(Component, Default)]
pub enum RoomObjectId {
    #[default]
    DietFridge,
    SeedChest,
    PushPopFeedingDish,
}

pub fn room_object_display_name(object_id: RoomObjectId) -> &'static str {
    match object_id {
        RoomObjectId::DietFridge => "Diet Fridge",
        RoomObjectId::SeedChest => "Seed Chest",
        RoomObjectId::PushPopFeedingDish => "Push Pop's Feeding Dish",
    }
}

// ---------------------------------------------------------
// Animal placement (runtime; positions change over time)
// ---------------------------------------------------------

#[derive(Debug, Clone, Copy)]
pub struct TileBounds {
    pub bottom_left: TilePosition,
    pub top_right: TilePosition,
}

#[derive(Debug, Clone, Copy)]
pub struct AnimalPlacementDef {
    pub animal_id: AnimalId,
    pub home_position: TilePosition,
    pub wander_bounds: TileBounds,
}

/// Sync with design/rooms/nutrition_house.json animals[0].
/// `home_position` is the default tile for new saves only — not the runtime spawn point.
pub const POLLY_PLACEMENT: AnimalPlacementDef = AnimalPlacementDef {
    animal_id: AnimalId::Polly,
    home_position: TilePosition { x: 8, y: 4 },
    wander_bounds: TileBounds {
        bottom_left: TilePosition { x: 7, y: 1 },
        top_right: TilePosition { x: 9, y: 5 },
    },
};

/// Sync with design/rooms/push_pop_enclosure.json animals[0].
/// `home_position` is the default tile for new saves only — not the runtime spawn point.
pub const PUSH_POP_PLACEMENT: AnimalPlacementDef = AnimalPlacementDef {
    animal_id: AnimalId::PushPop,
    home_position: TilePosition { x: 8, y: 4 },
    wander_bounds: TileBounds {
        bottom_left: TilePosition { x: 5, y: 3 },
        top_right: TilePosition { x: 10, y: 8 },
    },
};

pub fn animal_default_placement(animal_id: AnimalId) -> Option<&'static AnimalPlacementDef> {
    match animal_id {
        AnimalId::Polly => Some(&POLLY_PLACEMENT),
        AnimalId::PushPop => Some(&PUSH_POP_PLACEMENT),
        _ => None,
    }
}

pub fn enclosure_for_animal(animal_id: AnimalId) -> EnclosureId {
    match animal_id {
        AnimalId::Polly => EnclosureId::NutritionHousePlaypen,
        AnimalId::PushPop => EnclosureId::PushPopEnclosure,
        AnimalId::Stompy => EnclosureId::Pasture,
        AnimalId::Georgie | AnimalId::Siren => EnclosureId::ReptileEnclosure,
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

/// Rough idle wander rate used for offline catch-up (steps per hour).
pub const OFFLINE_WANDER_STEPS_PER_HOUR: f32 = 30.0;
