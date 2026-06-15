//! Hardcoded game content synced with `design/data/*.json` and `design/rooms/*.json`.
//! Do not load these files at runtime.

use bevy::prelude::*;
use crate::components::TilePosition;
use crate::stats::{AnimalId, AnimalStat};

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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Reflect)]
pub enum RoomObjectId {
    DietFridge,
    SeedChest,
    PushPopFeedingDish,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Reflect)]
pub enum InteractionKind {
    GiveItem {
        item_id: ItemId,
        prompt: &'static str,
    },
    FeedAnimal {
        animal_id: AnimalId,
        required_item: ItemId,
        stat: AnimalStat,
        delta: u32,
        prompt: &'static str,
    },
}

#[derive(Debug, Clone, Copy)]
pub struct RoomObjectDef {
    pub object_id: RoomObjectId,
    pub display_name: &'static str,
    pub position: TilePosition,
    pub is_obstacle: bool,
    pub interaction: Option<InteractionKind>,
    pub color: Color,
}

/// Sync with design/rooms/nutrition_house.json objects (MVP subset).
pub const NUTRITION_HOUSE_OBJECTS: &[RoomObjectDef] = &[
    RoomObjectDef {
        object_id: RoomObjectId::DietFridge,
        display_name: "Diet Fridge",
        position: TilePosition { x: 2, y: 8 },
        is_obstacle: true,
        interaction: Some(InteractionKind::GiveItem {
            item_id: ItemId::TortoiseLeafyGreens,
            prompt: "Scoop tortoise leafy greens",
        }),
        color: Color::srgb(0.75, 0.78, 0.80),
    },
    RoomObjectDef {
        object_id: RoomObjectId::SeedChest,
        display_name: "Seed Chest",
        position: TilePosition { x: 2, y: 5 },
        is_obstacle: true,
        interaction: Some(InteractionKind::GiveItem {
            item_id: ItemId::ChickenGrains,
            prompt: "Scoop chicken grains",
        }),
        color: Color::srgb(0.60, 0.40, 0.10),
    },
];

/// Sync with design/rooms/push_pop_enclosure.json objects.
pub const PUSH_POP_ENCLOSURE_OBJECTS: &[RoomObjectDef] = &[RoomObjectDef {
    object_id: RoomObjectId::PushPopFeedingDish,
    display_name: "Push Pop's Feeding Dish",
    position: TilePosition { x: 8, y: 6 },
    is_obstacle: true,
    interaction: Some(InteractionKind::FeedAnimal {
        animal_id: AnimalId::PushPop,
        required_item: ItemId::TortoiseLeafyGreens,
        stat: AnimalStat::Hunger,
        delta: 1000,
        prompt: "Place leafy greens for Push Pop",
    }),
    color: Color::srgb(0.55, 0.45, 0.30),
}];

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

/// Sync with design/rooms/push_pop_enclosure.json animals[0].
pub const PUSH_POP_PLACEMENT: AnimalPlacementDef = AnimalPlacementDef {
    animal_id: AnimalId::PushPop,
    home_position: TilePosition { x: 8, y: 4 },
    wander_bounds: TileBounds {
        bottom_left: TilePosition { x: 5, y: 3 },
        top_right: TilePosition { x: 10, y: 8 },
    },
};

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
