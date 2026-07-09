//! Static game-design tables synced with `design/data/*.json` and
//! `design/rooms/*.json`. These are the canonical runtime values.
//!
//! Types here are the config *schemas*; the identifier/value types they are
//! built from live in [`alveus_types`]. The main crate re-exports both so they
//! remain accessible under their original module paths.

use alveus_types::{AnimalId, EnclosureId, ItemId, TileBounds, TilePosition};

// ---------------------------------------------------------
// Items (sync with design/data/items.json)
// ---------------------------------------------------------

#[derive(Debug, Clone, Copy)]
pub struct ItemStaticData {
    pub display_name: &'static str,
}

/// Static data for an item. This is a *total* mapping — every [`ItemId`] has
/// data — so it's an exhaustive `match` rather than a partial lookup: adding an
/// `ItemId` variant fails to compile until its data is filled in, with no
/// runtime `Unknown Item` fallback.
pub const fn item_data(item_id: ItemId) -> ItemStaticData {
    match item_id {
        ItemId::TortoiseLeafyGreens => ItemStaticData {
            display_name: "Tortoise Leafy Greens",
        },
        ItemId::ChickenGrains => ItemStaticData {
            display_name: "Chicken Grains",
        },
    }
}

pub const fn item_display_name(item_id: ItemId) -> &'static str {
    item_data(item_id).display_name
}

// ---------------------------------------------------------
// Animal placement (runtime; positions change over time)
// ---------------------------------------------------------

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

/// Rough idle wander rate used for offline catch-up (steps per hour).
pub const OFFLINE_WANDER_STEPS_PER_HOUR: f32 = 30.0;

// ---------------------------------------------------------
// Cleaning / poop (sync with design/rooms/*.json dynamic_spawns)
// ---------------------------------------------------------

#[derive(Debug, Clone, Copy)]
pub struct PoopConfig {
    /// Cleanliness at or below each threshold (0–1000 scale) adds one poop on the floor.
    pub spawn_thresholds: &'static [u32],
    /// Extra cleanliness units lost per hour per poop on the floor (0–1000 scale).
    pub poop_decay_rate: f32,
    /// Cleanliness restored when a poop is picked up (not when the wheelbarrow is emptied).
    pub cleanliness_restore_per_poop: u32,
    pub spawn_bounds: TileBounds,
}

pub const WHEELBARROW_CAPACITY: u8 = 10;

const PUSH_POP_POOP_CONFIG: PoopConfig = PoopConfig {
    spawn_thresholds: &[800, 500, 200],
    poop_decay_rate: 20.0,
    cleanliness_restore_per_poop: 350,
    spawn_bounds: PUSH_POP_PLACEMENT.wander_bounds,
};

/// Static poop tuning for an enclosure.
///
/// Exhaustive over [`EnclosureId`]. Placeholder arms copy Push Pop's values for
/// now so adding a variant fails to compile until the table is consciously
/// extended.
pub const fn poop_config_for(id: EnclosureId) -> &'static PoopConfig {
    match id {
        EnclosureId::NutritionHousePlaypen => &PUSH_POP_POOP_CONFIG,
        EnclosureId::PushPopEnclosure => &PUSH_POP_POOP_CONFIG,
        EnclosureId::Pasture => &PUSH_POP_POOP_CONFIG,
        EnclosureId::ReptileEnclosure => &PUSH_POP_POOP_CONFIG,
    }
}

// ---------------------------------------------------------
// Cleaning math (pure functions over the poop tables)
// ---------------------------------------------------------

/// How many poops should be on the floor given current enclosure cleanliness.
pub fn target_poop_count(cleanliness: u32, thresholds: &[u32]) -> u32 {
    thresholds
        .iter()
        .filter(|&&threshold| cleanliness <= threshold)
        .count() as u32
}

/// Effective cleanliness decay rate accounting for poops currently on the floor.
pub fn cleanliness_decay_with_poops(
    base_rate: f32,
    enclosure_id: EnclosureId,
    poop_count: usize,
) -> f32 {
    let config = poop_config_for(enclosure_id);
    base_rate + config.poop_decay_rate * poop_count as f32
}

/// Simulate threshold-crossing poop acceleration over a block of hours (offline / time-skip).
pub fn cleanliness_after_threshold_decay(
    start: u32,
    hours: f32,
    base_rate: f32,
    config: &PoopConfig,
) -> u32 {
    if hours <= 0.0 {
        return start;
    }

    let mut current = start;
    let mut remaining = hours;

    let mut thresholds: Vec<u32> = config.spawn_thresholds.to_vec();
    thresholds.sort_by(|a, b| b.cmp(a));

    for &threshold in &thresholds {
        if current <= threshold {
            continue;
        }
        let poop_count = target_poop_count(current, config.spawn_thresholds);
        let rate = base_rate + config.poop_decay_rate * poop_count as f32;
        if rate <= 0.0 {
            break;
        }
        let drain_to_threshold = current - threshold;
        let time_needed = drain_to_threshold as f32 / rate;

        if time_needed <= remaining {
            remaining -= time_needed;
            current = threshold;
        } else {
            let decay = (rate * remaining).round() as u32;
            return current.saturating_sub(decay);
        }
    }

    if remaining > 0.0 && current > 0 {
        let poop_count = target_poop_count(current, config.spawn_thresholds);
        let rate = base_rate + config.poop_decay_rate * poop_count as f32;
        let decay = (rate * remaining).round() as u32;
        current = current.saturating_sub(decay);
    }

    current
}

/// Total cleanliness units lost over `hours`, accounting for threshold poop acceleration when configured.
pub fn enclosure_cleanliness_decay_amount(
    start: u32,
    hours: f32,
    base_rate: f32,
    enclosure_id: EnclosureId,
    _starting_poop_count: usize,
) -> u32 {
    if hours <= 0.0 {
        return 0;
    }
    let config = poop_config_for(enclosure_id);
    start.saturating_sub(cleanliness_after_threshold_decay(
        start, hours, base_rate, config,
    ))
}
