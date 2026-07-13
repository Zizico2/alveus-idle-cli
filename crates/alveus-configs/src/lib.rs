//! Canonical runtime gameplay numbers for alveus-idle.
//!
//! **This crate is the source of truth for shipped magic values.** Gameplay crates
//! read constants and tables from here — do not scatter new balance numbers in
//! feature crates. Not-yet-implemented ballparks live in this crate's `README.md`
//! (Planned defaults); promote them into Rust when a system ships.
//!
//! Identifier enums live in [`alveus_types`]. Historical prose lives in `design/`
//! (markdown only) and [`ROADMAP.md`](../../ROADMAP.md).

use alveus_types::{
    AnimalId, CareMenuId, ChoreId, CleanStat, EnclosureId, EnrichStat, FeedStat, ItemId, Stat,
    TileBounds, TilePosition,
};

// ---------------------------------------------------------
// Scale & timing
// ---------------------------------------------------------

/// Internal stat scale: design fractions `0.0–1.0` map to `0..=STAT_SCALE`.
pub const STAT_SCALE: Stat = Stat(1000);

/// Full / initial value for hunger, happiness, and enclosure cleanliness.
pub const STAT_FULL: Stat = STAT_SCALE;

/// World tile size in pixels.
pub const TILE_SIZE: u32 = 32;

/// Wall-clock seconds for one player tile step.
pub const PLAYER_MOVE_DURATION_SECS: f32 = 0.25;

/// Autosave interval (seconds).
pub const AUTOSAVE_INTERVAL_SECS: f32 = 5.0;

/// Wall-clock seconds before Loading aborts back to Title.
pub const LOADING_TIMEOUT_SECS: f32 = 15.0;

/// Debug-key stat improvement used by the local development input context.
pub const DEBUG_STAT_IMPROVE_AMOUNT: Stat = Stat(250);

/// Debug-key all-stat decrease used by the local development input context.
pub const DEBUG_STAT_WORSEN_AMOUNT: Stat = Stat(100);

/// Simulated hours advanced by the local development fast-forward shortcut.
pub const DEBUG_ADVANCE_HOURS: f32 = 4.0;

/// Upkeep score at or below this shows the neglect banner (and related Epic 5 effects).
pub const NEGLECT_UPKEEP_THRESHOLD: f32 = 0.30;

/// Caretaker satchel capacity (two carried items).
pub const SATCHEL_MAX_SLOTS: u8 = 2;

/// Typical feed restore amount ([`STAT_FULL`] = full bar).
pub const CARE_FEED_RESTORE: FeedStat = FeedStat(STAT_FULL);

/// Typical enrichment (happiness) restore amount.
pub const CARE_ENRICH_RESTORE: EnrichStat = EnrichStat(STAT_FULL);

/// Typical clean / nesting restore amount (enclosure cleanliness).
pub const CARE_CLEAN_RESTORE: CleanStat = CleanStat(STAT_FULL);

/// Default overview spawn when entering gameplay (runtime; not design-map coords).
pub const OVERVIEW_PLAYER_SPAWN: TilePosition = TilePosition { x: 0, y: 0 };

/// Rough idle wander rate used for offline catch-up (steps per hour).
pub const OFFLINE_WANDER_STEPS_PER_HOUR: f32 = 30.0;

pub const WHEELBARROW_CAPACITY: u8 = 10;

// ---------------------------------------------------------
// Items
// ---------------------------------------------------------

#[derive(Debug, Clone, Copy)]
pub struct ItemStaticData {
    pub display_name: &'static str,
}

/// Exhaustive item table — adding an [`ItemId`] fails to compile until filled in.
pub const fn item_data(item_id: ItemId) -> ItemStaticData {
    match item_id {
        ItemId::TortoiseLeafyGreens => ItemStaticData {
            display_name: "Tortoise Leafy Greens",
        },
        ItemId::ChickenGrains => ItemStaticData {
            display_name: "Chicken Grains",
        },
        ItemId::RawVeggieTub => ItemStaticData {
            display_name: "Lettuce & Veggie Tub",
        },
        ItemId::PreparedVeggieDiet => ItemStaticData {
            display_name: "Prepared Veggie Diet",
        },
        ItemId::MiniMirror => ItemStaticData {
            display_name: "Mini Mirror",
        },
    }
}

pub const fn item_display_name(item_id: ItemId) -> &'static str {
    item_data(item_id).display_name
}

/// Prep recipe: input item + chore → output item.
#[derive(Debug, Clone, Copy)]
pub struct PrepRecipe {
    pub chore_id: ChoreId,
    pub input: ItemId,
    pub output: ItemId,
}

pub const PREP_RECIPES: &[PrepRecipe] = &[PrepRecipe {
    chore_id: ChoreId::ChopVeggies,
    input: ItemId::RawVeggieTub,
    output: ItemId::PreparedVeggieDiet,
}];

pub fn prep_recipe_for(chore_id: ChoreId, input: ItemId) -> Option<&'static PrepRecipe> {
    PREP_RECIPES
        .iter()
        .find(|recipe| recipe.chore_id == chore_id && recipe.input == input)
}

/// Items offered by a care item-picker menu.
pub const fn care_menu_options(menu_id: CareMenuId) -> &'static [ItemId] {
    match menu_id {
        CareMenuId::Fridge => &[ItemId::RawVeggieTub, ItemId::TortoiseLeafyGreens],
    }
}

// ---------------------------------------------------------
// Animals
// ---------------------------------------------------------

#[derive(Debug, Clone, Copy)]
pub struct AnimalStaticData {
    pub animal_id: AnimalId,
    pub display_name: &'static str,
    /// Common species label for HUD / education hooks.
    pub species: &'static str,
    /// Short home / enclosure label for HUD cards.
    pub home_label: &'static str,
    /// Hunger decay as a fraction of [`STAT_SCALE`] per simulated hour (e.g. `0.04`).
    pub hunger_decay_rate: f32,
    /// Happiness decay as a fraction of [`STAT_SCALE`] per simulated hour.
    pub happiness_decay_rate: f32,
}

pub const ANIMALS_DATA: &[AnimalStaticData] = &[
    AnimalStaticData {
        animal_id: AnimalId::Polly,
        display_name: "Polly",
        species: "Silkie Chicken",
        home_label: "Playpen",
        hunger_decay_rate: 0.04,
        happiness_decay_rate: 0.05,
    },
    AnimalStaticData {
        animal_id: AnimalId::PushPop,
        display_name: "Push Pop",
        species: "Sulcata Tortoise",
        home_label: "Push Pop Enclosure",
        hunger_decay_rate: 0.04,
        happiness_decay_rate: 0.05,
    },
    AnimalStaticData {
        animal_id: AnimalId::Stompy,
        display_name: "Stompy",
        species: "Emu",
        home_label: "Pasture Grassland",
        hunger_decay_rate: 0.04,
        happiness_decay_rate: 0.05,
    },
    AnimalStaticData {
        animal_id: AnimalId::Georgie,
        display_name: "Georgie",
        species: "African Bullfrog",
        home_label: "Studio",
        hunger_decay_rate: 0.04,
        happiness_decay_rate: 0.05,
    },
    AnimalStaticData {
        animal_id: AnimalId::Siren,
        display_name: "Siren",
        species: "Blue-fronted Amazon",
        home_label: "Studio",
        hunger_decay_rate: 0.04,
        happiness_decay_rate: 0.05,
    },
];

/// Exhaustive lookup — prefer this when matching on a single id.
pub const fn animal_data(animal_id: AnimalId) -> &'static AnimalStaticData {
    match animal_id {
        AnimalId::Polly => &ANIMALS_DATA[0],
        AnimalId::PushPop => &ANIMALS_DATA[1],
        AnimalId::Stompy => &ANIMALS_DATA[2],
        AnimalId::Georgie => &ANIMALS_DATA[3],
        AnimalId::Siren => &ANIMALS_DATA[4],
    }
}

pub const fn enclosure_for_animal(animal_id: AnimalId) -> EnclosureId {
    match animal_id {
        AnimalId::Polly => EnclosureId::NutritionHousePlaypen,
        AnimalId::PushPop => EnclosureId::PushPopEnclosure,
        AnimalId::Stompy => EnclosureId::Pasture,
        AnimalId::Georgie | AnimalId::Siren => EnclosureId::ReptileEnclosure,
    }
}

// ---------------------------------------------------------
// Enclosures
// ---------------------------------------------------------

#[derive(Debug, Clone, Copy)]
pub struct EnclosureStaticData {
    pub enclosure_id: EnclosureId,
    pub display_name: &'static str,
    /// Cleanliness units lost per hour on the [`STAT_SCALE`] integer scale.
    pub cleanliness_decay_per_hour: f32,
}

pub const ENCLOSURES_DATA: &[EnclosureStaticData] = &[
    EnclosureStaticData {
        enclosure_id: EnclosureId::NutritionHousePlaypen,
        display_name: "Nutrition House Playpen",
        cleanliness_decay_per_hour: 0.03 * STAT_SCALE.get() as f32,
    },
    EnclosureStaticData {
        enclosure_id: EnclosureId::PushPopEnclosure,
        display_name: "Push Pop Enclosure",
        cleanliness_decay_per_hour: 0.03 * STAT_SCALE.get() as f32,
    },
    EnclosureStaticData {
        enclosure_id: EnclosureId::Pasture,
        display_name: "Pasture Grassland",
        cleanliness_decay_per_hour: 0.03 * STAT_SCALE.get() as f32,
    },
    EnclosureStaticData {
        enclosure_id: EnclosureId::ReptileEnclosure,
        display_name: "Reptile Enclosure",
        cleanliness_decay_per_hour: 0.03 * STAT_SCALE.get() as f32,
    },
];

pub const fn enclosure_data(enclosure_id: EnclosureId) -> &'static EnclosureStaticData {
    match enclosure_id {
        EnclosureId::NutritionHousePlaypen => &ENCLOSURES_DATA[0],
        EnclosureId::PushPopEnclosure => &ENCLOSURES_DATA[1],
        EnclosureId::Pasture => &ENCLOSURES_DATA[2],
        EnclosureId::ReptileEnclosure => &ENCLOSURES_DATA[3],
    }
}

// ---------------------------------------------------------
// Animal placement (defaults for new saves)
// ---------------------------------------------------------

#[derive(Debug, Clone, Copy)]
pub struct AnimalPlacementDef {
    pub animal_id: AnimalId,
    pub home_position: TilePosition,
    pub wander_bounds: TileBounds,
}

/// `home_position` is the default tile for new saves only — not the runtime spawn point.
/// Wander confined to Polly's playpen in the east corner of the Nutrition House.
pub const POLLY_PLACEMENT: AnimalPlacementDef = AnimalPlacementDef {
    animal_id: AnimalId::Polly,
    home_position: TilePosition { x: 8, y: 4 },
    wander_bounds: TileBounds {
        bottom_left: TilePosition { x: 6, y: 1 },
        top_right: TilePosition { x: 9, y: 5 },
    },
};

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

// ---------------------------------------------------------
// Room tile configs (implemented interiors)
// ---------------------------------------------------------

#[derive(Debug, Clone, Copy)]
pub struct RoomTileConfig {
    pub room_spawn: TilePosition,
    pub exit_spawn: TilePosition,
    pub exit_door: TilePosition,
}

pub const NUTRITION_HOUSE_ROOM: RoomTileConfig = RoomTileConfig {
    room_spawn: TilePosition { x: 5, y: 2 },
    exit_spawn: TilePosition { x: 33, y: 12 },
    exit_door: TilePosition { x: 5, y: 0 },
};

pub const PUSH_POP_ENCLOSURE_ROOM: RoomTileConfig = RoomTileConfig {
    room_spawn: TilePosition { x: 6, y: 2 },
    exit_spawn: TilePosition { x: 40, y: 33 },
    exit_door: TilePosition { x: 6, y: 0 },
};

// ---------------------------------------------------------
// Cleaning / poop
// ---------------------------------------------------------

#[derive(Debug, Clone, Copy)]
pub struct PoopConfig {
    /// Cleanliness at or below each threshold ([`STAT_SCALE`] units) adds one poop.
    pub spawn_thresholds: &'static [Stat],
    /// Extra cleanliness units lost per hour per poop on the floor.
    pub poop_decay_rate: f32,
    /// Cleanliness restored when a poop is picked up (not when the wheelbarrow is emptied).
    pub cleanliness_restore_per_poop: CleanStat,
    pub spawn_bounds: TileBounds,
}

include!(concat!(env!("OUT_DIR"), "/poop_configs.rs"));

/// Poop pile config when this enclosure participates in pile-based cleaning.
///
/// Configs are sparse and generated by `build.rs` via `phf_codegen`; absence
/// means nest-sweep / manure-free care (e.g. Nutrition House).
pub fn poop_config_for(id: EnclosureId) -> Option<&'static PoopConfig> {
    POOP_CONFIGS.get(&id)
}

// ---------------------------------------------------------
// Cleaning math
// ---------------------------------------------------------

/// How many poops should be on the floor given current enclosure cleanliness.
pub fn target_poop_count(cleanliness: Stat, thresholds: &[Stat]) -> u32 {
    thresholds
        .iter()
        .filter(|&&threshold| cleanliness <= threshold)
        .count() as u32
}

/// Effective cleanliness decay rate accounting for poops currently on the floor.
/// Enclosures without a poop config decay at `base_rate` only.
pub fn cleanliness_decay_with_poops(
    base_rate: f32,
    enclosure_id: EnclosureId,
    poop_count: usize,
) -> f32 {
    match poop_config_for(enclosure_id) {
        Some(config) => base_rate + config.poop_decay_rate * poop_count as f32,
        None => base_rate,
    }
}

/// Simulate threshold-crossing poop acceleration over a block of hours (offline / time-skip).
pub fn cleanliness_after_threshold_decay(
    start: Stat,
    hours: f32,
    base_rate: f32,
    config: &PoopConfig,
) -> Stat {
    if hours <= 0.0 {
        return start;
    }

    let mut current = start;
    let mut remaining = hours;

    let mut thresholds: Vec<Stat> = config.spawn_thresholds.to_vec();
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
        let drain_to_threshold = current.get() - threshold.get();
        let time_needed = drain_to_threshold as f32 / rate;

        if time_needed <= remaining {
            remaining -= time_needed;
            current = threshold;
        } else {
            let decay = (rate * remaining).round() as u32;
            return current.saturating_sub(Stat(decay));
        }
    }

    if remaining > 0.0 && !current.is_zero() {
        let poop_count = target_poop_count(current, config.spawn_thresholds);
        let rate = base_rate + config.poop_decay_rate * poop_count as f32;
        let decay = (rate * remaining).round() as u32;
        current = current.saturating_sub(Stat(decay));
    }

    current
}

/// Total cleanliness units lost over `hours`, accounting for threshold poop acceleration.
/// Without a poop config, applies plain `base_rate * hours` decay.
pub fn enclosure_cleanliness_decay_amount(
    start: Stat,
    hours: f32,
    base_rate: f32,
    enclosure_id: EnclosureId,
    _starting_poop_count: usize,
) -> Stat {
    if hours <= 0.0 {
        return Stat::ZERO;
    }
    match poop_config_for(enclosure_id) {
        Some(config) => start.saturating_sub(cleanliness_after_threshold_decay(
            start, hours, base_rate, config,
        )),
        None => {
            let decay = (base_rate * hours).round() as u32;
            Stat(decay.min(start.get()))
        }
    }
}
