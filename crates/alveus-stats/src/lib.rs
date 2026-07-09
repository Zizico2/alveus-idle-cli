use alveus_app::AppSystems;
use alveus_collision::{
    CollisionMapKey, CollisionMasks, DynamicObstacleTiles, LiveObstacleItem, random_wander_step,
};
use alveus_components::PoopWheelbarrow;
use alveus_configs::{
    ANIMALS_DATA, AUTOSAVE_INTERVAL_SECS, ENCLOSURES_DATA, OFFLINE_WANDER_STEPS_PER_HOUR, STAT_FULL,
    STAT_SCALE, animal_default_placement, cleanliness_decay_with_poops,
    enclosure_cleanliness_decay_amount, enclosure_for_animal,
};
use alveus_content::default_tile_position;
pub use alveus_types::{AnimalId, EnclosureId};
use alveus_types::{TileBounds, TilePosition};
use bevy::prelude::*;
use moonshine_save::prelude::*;
use rand::rng;
use std::collections::HashSet;
use std::path::Path;
use std::time::SystemTime;

use alveus_app::Screen;

// ---------------------------------------------------------
// Components
// ---------------------------------------------------------

#[derive(Component, Reflect, Default)]
#[reflect(Component)]
#[require(Save, Unload)]
pub struct SaveTimestamp {
    pub value: u64,
}

#[derive(Component, Debug, Clone, Reflect)]
#[reflect(Component)]
pub struct AnimalName(pub String);

#[derive(Component, Debug, Clone, Reflect)]
#[reflect(Component)]
pub struct AnimalStats {
    pub hunger: u32,    // [0, STAT_SCALE]
    pub happiness: u32, // [0, STAT_SCALE]
}

#[derive(Component, Debug, Clone, Reflect)]
#[reflect(Component)]
pub struct AnimalDecayRates {
    pub hunger_rate: f32,    // units per hour
    pub happiness_rate: f32, // units per hour
}

#[derive(Component, Debug, Clone, Reflect, Default)]
#[reflect(Component)]
pub struct AnimalDecayAccumulators {
    pub hunger: f32,
    pub happiness: f32,
}

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Hash, Reflect, Default)]
#[reflect(Component)]
pub struct AnimalEnclosure(pub EnclosureId);

#[derive(Component, Debug, Clone, Copy, Reflect)]
#[reflect(Component)]
#[require(Save, Unload)]
pub struct AnimalTilePosition(pub TilePosition);

#[derive(Component, Debug, Clone)]
pub struct AnimalBackgroundWander {
    pub bounds: TileBounds,
    pub idle_timer: Timer,
}

impl AnimalBackgroundWander {
    pub fn new(bounds: TileBounds) -> Self {
        Self {
            bounds,
            idle_timer: Timer::from_seconds(2.0, TimerMode::Repeating),
        }
    }
}

#[derive(Component, Debug, Clone, Reflect)]
#[reflect(Component)]
pub struct EnclosureName(pub String);

#[derive(Component, Debug, Clone, Reflect)]
#[reflect(Component)]
pub struct EnclosureStats {
    pub cleanliness: u32, // [0, STAT_SCALE]
}

#[derive(Component, Debug, Clone, Reflect)]
#[reflect(Component)]
pub struct EnclosureDecayRates {
    pub cleanliness_rate: f32, // units per hour
}

#[derive(Component, Debug, Clone, Reflect, Default)]
#[reflect(Component)]
pub struct EnclosureDecayAccumulators {
    pub cleanliness: f32,
}

// ---------------------------------------------------------
// Events & Enums
// ---------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Reflect)]
pub enum AnimalStat {
    Hunger,
    Happiness,
    Cleanliness,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Reflect)]
pub enum EnclosureStat {
    Cleanliness,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Reflect)]
pub enum StatTarget {
    Animal {
        id: AnimalId,
        stat: AnimalStat,
    },
    Enclosure {
        id: EnclosureId,
        stat: EnclosureStat,
    },
}

#[derive(Event, Debug, Clone, Copy, Reflect)]
#[reflect(Event)]
pub struct ImproveStatEvent {
    pub target: StatTarget,
    pub amount: u32,
}

#[derive(Event, Debug, Clone, Copy, Reflect)]
#[reflect(Event)]
pub struct WorsenStatEvent {
    pub target: StatTarget,
    pub amount: u32,
}

// ---------------------------------------------------------
// Resources
// ---------------------------------------------------------

#[derive(Resource, Debug, Default, Clone, Copy, Reflect)]
#[reflect(Resource)]
pub struct SanctuaryUpkeep {
    pub score: f32,
    pub mean_hunger: f32,
    pub mean_cleanliness: f32,
    pub mean_happiness: f32,
}

/// Offline wander steps to apply once collision masks are ready.
#[derive(Resource, Debug, Default)]
pub struct PendingOfflineWander {
    pub steps: u32,
}

fn animal_world_components(
    animal_id: AnimalId,
) -> (Option<AnimalTilePosition>, Option<AnimalBackgroundWander>) {
    match animal_default_placement(animal_id) {
        Some(placement) => (
            Some(AnimalTilePosition(placement.home_position)),
            Some(AnimalBackgroundWander::new(placement.wander_bounds)),
        ),
        None => (None, None),
    }
}

// ---------------------------------------------------------
// Plugin
// ---------------------------------------------------------

#[derive(Resource, Reflect)]
#[reflect(Resource)]
pub struct SavePath(pub String);

impl Default for SavePath {
    fn default() -> Self {
        Self("save.ron".to_string())
    }
}

pub struct StatsPlugin;

impl Plugin for StatsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SavePath>()
            .register_type::<SaveTimestamp>()
            .register_type::<AnimalId>()
            .register_type::<AnimalStat>()
            .register_type::<AnimalName>()
            .register_type::<AnimalStats>()
            .register_type::<AnimalDecayRates>()
            .register_type::<AnimalDecayAccumulators>()
            .register_type::<AnimalEnclosure>()
            .register_type::<AnimalTilePosition>()
            .register_type::<DynamicObstacleTiles>()
            .register_type::<EnclosureId>()
            .register_type::<EnclosureName>()
            .register_type::<EnclosureStats>()
            .register_type::<EnclosureDecayRates>()
            .register_type::<EnclosureDecayAccumulators>()
            .register_type::<SanctuaryUpkeep>()
            .init_resource::<SanctuaryUpkeep>()
            .init_resource::<PendingOfflineWander>()
            .init_resource::<AutoSaveTimer>()
            .init_resource::<DebugLogTimer>()
            .add_observer(save_on_default_event)
            .add_observer(load_on_default_event)
            .add_observer(hydrate_loaded_stats_observer)
            .add_systems(Update, ensure_animal_world_state)
            .add_systems(Update, ensure_dynamic_obstacle_tiles)
            // Register decoupled observers
            .add_observer(improve_stat_observer)
            .add_observer(worsen_stat_observer)
            // Startup / Initialization when entering gameplay
            .add_systems(OnEnter(Screen::Gameplay), init_stats_system);

        // Update systems run when player is actively playing
        app.add_systems(
            Update,
            (
                tick_decay_system.in_set(AppSystems::DecayCalculation),
                update_upkeep_system.in_set(AppSystems::UpkeepCalculation),
                debug_stats_control_system,
                save_stats_periodically_system.in_set(AppSystems::SaveSystem),
                debug_log_stats_system,
            )
                .run_if(in_gameplay_or_room),
        );
        app.add_systems(
            Update,
            apply_offline_decay_system
                .in_set(AppSystems::DecayCalculation)
                .run_if(in_gameplay_or_room)
                .run_if(any_with_component::<SaveTimestamp>),
        );
        app.add_systems(
            Update,
            apply_offline_wander_system
                .in_set(AppSystems::DecayCalculation)
                .run_if(in_gameplay_or_room)
                .run_if(|pending: Res<PendingOfflineWander>| pending.steps > 0),
        );
    }
}

// ---------------------------------------------------------
// Run Conditions & Helpers
// ---------------------------------------------------------

fn in_gameplay_or_room(screen_state: Res<State<Screen>>) -> bool {
    matches!(
        screen_state.get(),
        Screen::Gameplay | Screen::InRoom(_)
    )
}

// ---------------------------------------------------------
// Observers (Decoupled Stat Modification)
// ---------------------------------------------------------

fn improve_stat_observer(
    trigger: On<ImproveStatEvent>,
    mut animal_query: Query<(&AnimalId, &AnimalName, &mut AnimalStats, &AnimalEnclosure)>,
    mut enclosure_query: Query<(&EnclosureId, &EnclosureName, &mut EnclosureStats)>,
) {
    let event = trigger.event();
    info!(
        "improve_stat_observer triggered for target '{:?}', amount {}",
        event.target, event.amount
    );

    match event.target {
        StatTarget::Animal {
            id: target_animal_id,
            stat,
        } => match stat {
            AnimalStat::Hunger | AnimalStat::Happiness => {
                let mut found = false;
                for (id, name, mut stats, _) in &mut animal_query {
                    if *id == target_animal_id {
                        found = true;
                        match stat {
                            AnimalStat::Hunger => {
                                let prev = stats.hunger;
                                stats.hunger = stats.hunger.saturating_add(event.amount).min(STAT_SCALE);
                                info!(
                                    "Improved hunger for {} ({}): {} -> {}",
                                    name.0,
                                    id.as_str(),
                                    prev,
                                    stats.hunger
                                );
                            }
                            AnimalStat::Happiness => {
                                let prev = stats.happiness;
                                stats.happiness =
                                    stats.happiness.saturating_add(event.amount).min(STAT_SCALE);
                                info!(
                                    "Improved happiness for {} ({}): {} -> {}",
                                    name.0,
                                    id.as_str(),
                                    prev,
                                    stats.happiness
                                );
                            }
                            _ => unreachable!(),
                        }
                    }
                }
                if !found {
                    warn!(
                        "ImproveStatEvent: Animal id '{:?}' not found in world",
                        target_animal_id
                    );
                }
            }
            AnimalStat::Cleanliness => {
                let mut target_enclosure = None;
                for (id, _, _, animal_enc) in &animal_query {
                    if *id == target_animal_id {
                        target_enclosure = Some(animal_enc.0);
                        break;
                    }
                }

                if let Some(target_enc_id) = target_enclosure {
                    let mut found = false;
                    for (enc_id, enc_name, mut enc_stats) in &mut enclosure_query {
                        if *enc_id == target_enc_id {
                            found = true;
                            let prev = enc_stats.cleanliness;
                            enc_stats.cleanliness =
                                enc_stats.cleanliness.saturating_add(event.amount).min(STAT_SCALE);
                            info!(
                                "Improved cleanliness for Enclosure {} ({}): {} -> {}",
                                enc_name.0,
                                enc_id.as_str(),
                                prev,
                                enc_stats.cleanliness
                            );
                        }
                    }
                    if !found {
                        warn!(
                            "ImproveStatEvent: Enclosure id '{:?}' not found in world",
                            target_enc_id
                        );
                    }
                } else {
                    warn!(
                        "ImproveStatEvent: Enclosure for Animal id '{:?}' not found",
                        target_animal_id
                    );
                }
            }
        },
        StatTarget::Enclosure {
            id: target_enc_id,
            stat: EnclosureStat::Cleanliness,
        } => {
            let mut found = false;
            for (enc_id, enc_name, mut enc_stats) in &mut enclosure_query {
                if *enc_id == target_enc_id {
                    found = true;
                    let prev = enc_stats.cleanliness;
                    enc_stats.cleanliness =
                        enc_stats.cleanliness.saturating_add(event.amount).min(STAT_SCALE);
                    info!(
                        "Improved cleanliness for Enclosure {} ({}): {} -> {}",
                        enc_name.0,
                        enc_id.as_str(),
                        prev,
                        enc_stats.cleanliness
                    );
                }
            }
            if !found {
                warn!(
                    "ImproveStatEvent: Enclosure id '{:?}' not found in world",
                    target_enc_id
                );
            }
        }
    }
}

fn worsen_stat_observer(
    trigger: On<WorsenStatEvent>,
    mut animal_query: Query<(&AnimalId, &AnimalName, &mut AnimalStats, &AnimalEnclosure)>,
    mut enclosure_query: Query<(&EnclosureId, &EnclosureName, &mut EnclosureStats)>,
) {
    let event = trigger.event();
    info!(
        "worsen_stat_observer triggered for target '{:?}', amount {}",
        event.target, event.amount
    );

    match event.target {
        StatTarget::Animal {
            id: target_animal_id,
            stat,
        } => match stat {
            AnimalStat::Hunger | AnimalStat::Happiness => {
                let mut found = false;
                for (id, name, mut stats, _) in &mut animal_query {
                    if *id == target_animal_id {
                        found = true;
                        match stat {
                            AnimalStat::Hunger => {
                                let prev = stats.hunger;
                                stats.hunger = stats.hunger.saturating_sub(event.amount);
                                info!(
                                    "Worsened hunger for {} ({}): {} -> {}",
                                    name.0,
                                    id.as_str(),
                                    prev,
                                    stats.hunger
                                );
                            }
                            AnimalStat::Happiness => {
                                let prev = stats.happiness;
                                stats.happiness = stats.happiness.saturating_sub(event.amount);
                                info!(
                                    "Worsened happiness for {} ({}): {} -> {}",
                                    name.0,
                                    id.as_str(),
                                    prev,
                                    stats.happiness
                                );
                            }
                            _ => unreachable!(),
                        }
                    }
                }
                if !found {
                    warn!(
                        "WorsenStatEvent: Animal id '{:?}' not found in world",
                        target_animal_id
                    );
                }
            }
            AnimalStat::Cleanliness => {
                let mut target_enclosure = None;
                for (id, _, _, animal_enc) in &animal_query {
                    if *id == target_animal_id {
                        target_enclosure = Some(animal_enc.0);
                        break;
                    }
                }

                if let Some(target_enc_id) = target_enclosure {
                    let mut found = false;
                    for (enc_id, enc_name, mut enc_stats) in &mut enclosure_query {
                        if *enc_id == target_enc_id {
                            found = true;
                            let prev = enc_stats.cleanliness;
                            enc_stats.cleanliness =
                                enc_stats.cleanliness.saturating_sub(event.amount);
                            info!(
                                "Worsened cleanliness for Enclosure {} ({}): {} -> {}",
                                enc_name.0,
                                enc_id.as_str(),
                                prev,
                                enc_stats.cleanliness
                            );
                        }
                    }
                    if !found {
                        warn!(
                            "WorsenStatEvent: Enclosure id '{:?}' not found in world",
                            target_enc_id
                        );
                    }
                } else {
                    warn!(
                        "WorsenStatEvent: Enclosure for Animal id '{:?}' not found",
                        target_animal_id
                    );
                }
            }
        },
        StatTarget::Enclosure {
            id: target_enc_id,
            stat: EnclosureStat::Cleanliness,
        } => {
            let mut found = false;
            for (enc_id, enc_name, mut enc_stats) in &mut enclosure_query {
                if *enc_id == target_enc_id {
                    found = true;
                    let prev = enc_stats.cleanliness;
                    enc_stats.cleanliness = enc_stats.cleanliness.saturating_sub(event.amount);
                    info!(
                        "Worsened cleanliness for Enclosure {} ({}): {} -> {}",
                        enc_name.0,
                        enc_id.as_str(),
                        prev,
                        enc_stats.cleanliness
                    );
                }
            }
            if !found {
                warn!(
                    "WorsenStatEvent: Enclosure id '{:?}' not found in world",
                    target_enc_id
                );
            }
        }
    }
}

// ---------------------------------------------------------
// Systems
// ---------------------------------------------------------

fn is_valid_save_format(content: &str) -> bool {
    if let Ok(ron::Value::Map(map)) = ron::from_str::<ron::Value>(content) {
        for key in map.keys() {
            if let ron::Value::String(s) = key
                && (s == "resources" || s == "entities")
            {
                return true;
            }
        }
    }
    false
}

/// Initialize animal stats on entering gameplay screen, checking save data.
fn init_stats_system(mut commands: Commands, query: Query<&AnimalId>, save_path: Res<SavePath>) {
    // If stats entities already exist, don't spawn them again
    if !query.is_empty() {
        return;
    }

    info!("Initializing animal stats entities...");

    if Path::new(&save_path.0).exists() {
        let is_valid = if let Ok(content) = std::fs::read_to_string(&save_path.0) {
            is_valid_save_format(&content)
        } else {
            false
        };

        if is_valid {
            info!("Loading save from {}...", save_path.0);
            commands.trigger_load(LoadWorld::default_from_file(&save_path.0));
            return;
        } else {
            warn!(
                "Save file {} is in an invalid/legacy format. Fallback to default stats.",
                save_path.0
            );
        }
    }

    info!("No save file found (or invalid format). Initializing new stats.");
    spawn_default_stats(&mut commands);
}

fn spawn_default_stats(commands: &mut Commands) {
    for enc in ENCLOSURES_DATA {
        commands.spawn((
            Name::new(format!("Enclosure Stats - {}", enc.display_name)),
            enc.enclosure_id,
            EnclosureName(enc.display_name.to_string()),
            EnclosureStats {
                cleanliness: STAT_FULL,
            },
            EnclosureDecayRates {
                cleanliness_rate: enc.cleanliness_decay_per_hour,
            },
            EnclosureDecayAccumulators::default(),
            DynamicObstacleTiles::default(),
        ));
    }

    for animal in ANIMALS_DATA {
        let display_name = animal.display_name.to_string();

        let decay_rates = AnimalDecayRates {
            hunger_rate: animal.hunger_decay_rate * STAT_SCALE as f32,
            happiness_rate: animal.happiness_decay_rate * STAT_SCALE as f32,
        };

        let enc_id = enclosure_for_animal(animal.animal_id);
        let (tile_position, background_wander) = animal_world_components(animal.animal_id);

        let mut entity = commands.spawn((
            Name::new(format!("Persistent Stats - {}", display_name)),
            animal.animal_id,
            AnimalName(display_name),
            AnimalEnclosure(enc_id),
            AnimalStats {
                hunger: STAT_FULL,
                happiness: STAT_FULL,
            },
            decay_rates,
            AnimalDecayAccumulators::default(),
        ));

        if let Some(pos) = tile_position {
            entity.insert(pos);
        }
        if let Some(wander) = background_wander {
            entity.insert(wander);
        }
    }
}

fn spawn_missing_stat_entities(
    commands: &mut Commands,
    animal_query: &Query<(Entity, &AnimalId)>,
    enclosure_query: &Query<(Entity, &EnclosureId)>,
) {
    let existing_animals: HashSet<AnimalId> = animal_query.iter().map(|(_, id)| *id).collect();
    let existing_enclosures: HashSet<EnclosureId> =
        enclosure_query.iter().map(|(_, id)| *id).collect();

    for enc in ENCLOSURES_DATA {
        if existing_enclosures.contains(&enc.enclosure_id) {
            continue;
        }
        info!("Spawning missing enclosure stats entity: {}", enc.display_name);
        commands.spawn((
            Name::new(format!("Enclosure Stats - {}", enc.display_name)),
            enc.enclosure_id,
            EnclosureName(enc.display_name.to_string()),
            EnclosureStats {
                cleanliness: STAT_FULL,
            },
            EnclosureDecayRates {
                cleanliness_rate: enc.cleanliness_decay_per_hour,
            },
            EnclosureDecayAccumulators::default(),
            DynamicObstacleTiles::default(),
        ));
    }

    for animal in ANIMALS_DATA {
        if existing_animals.contains(&animal.animal_id) {
            continue;
        }
        info!(
            "Spawning missing animal stats entity: {}",
            animal.display_name
        );
        let decay_rates = AnimalDecayRates {
            hunger_rate: animal.hunger_decay_rate * STAT_SCALE as f32,
            happiness_rate: animal.happiness_decay_rate * STAT_SCALE as f32,
        };
        let (tile_position, background_wander) = animal_world_components(animal.animal_id);

        let mut entity = commands.spawn((
            Name::new(format!("Persistent Stats - {}", animal.display_name)),
            animal.animal_id,
            AnimalName(animal.display_name.to_string()),
            AnimalEnclosure(enclosure_for_animal(animal.animal_id)),
            AnimalStats {
                hunger: STAT_FULL,
                happiness: STAT_FULL,
            },
            decay_rates,
            AnimalDecayAccumulators::default(),
        ));

        if let Some(pos) = tile_position {
            entity.insert(pos);
        }
        if let Some(wander) = background_wander {
            entity.insert(wander);
        }
    }
}

fn hydrate_loaded_stats_observer(
    _trigger: On<Loaded>,
    mut commands: Commands,
    animal_query: Query<(Entity, &AnimalId)>,
    enclosure_query: Query<(Entity, &EnclosureId)>,
) {
    info!("Hydrating loaded stats with static config...");

    for (entity, id) in &animal_query {
        if let Some(animal) = ANIMALS_DATA.iter().find(|a| a.animal_id == *id) {
            let decay_rates = AnimalDecayRates {
                hunger_rate: animal.hunger_decay_rate * STAT_SCALE as f32,
                happiness_rate: animal.happiness_decay_rate * STAT_SCALE as f32,
            };

            let enc_id = enclosure_for_animal(*id);

            commands.entity(entity).insert((
                Name::new(format!("Persistent Stats - {}", animal.display_name)),
                AnimalName(animal.display_name.to_string()),
                AnimalEnclosure(enc_id),
                decay_rates,
                AnimalDecayAccumulators::default(),
            ));
        }
    }

    for (entity, id) in &enclosure_query {
        if let Some(enc) = ENCLOSURES_DATA
            .iter()
            .find(|mapped| mapped.enclosure_id == *id)
        {
            commands.entity(entity).insert((
                Name::new(format!("Enclosure Stats - {}", enc.display_name)),
                EnclosureName(enc.display_name.to_string()),
                EnclosureDecayRates {
                    cleanliness_rate: enc.cleanliness_decay_per_hour,
                },
                EnclosureDecayAccumulators::default(),
                DynamicObstacleTiles::default(),
            ));
        }
    }

    spawn_missing_stat_entities(&mut commands, &animal_query, &enclosure_query);
}

fn ensure_animal_world_state(
    mut commands: Commands,
    missing_position: Query<(Entity, &AnimalId), Without<AnimalTilePosition>>,
    missing_wander: Query<
        (Entity, &AnimalId),
        (With<AnimalTilePosition>, Without<AnimalBackgroundWander>),
    >,
) {
    for (entity, id) in &missing_position {
        if let Some(pos) = default_tile_position(*id) {
            commands.entity(entity).insert(AnimalTilePosition(pos));
        }
    }

    for (entity, id) in &missing_wander {
        if let Some(placement) = alveus_content::animal_default_placement(*id) {
            commands
                .entity(entity)
                .insert(AnimalBackgroundWander::new(placement.wander_bounds));
        }
    }
}

fn ensure_dynamic_obstacle_tiles(
    mut commands: Commands,
    missing: Query<(Entity, &EnclosureId), Without<DynamicObstacleTiles>>,
) {
    for (entity, _) in &missing {
        commands
            .entity(entity)
            .insert(DynamicObstacleTiles::default());
    }
}

pub fn apply_offline_decay_system(
    mut commands: Commands,
    timestamp_query: Query<(Entity, &SaveTimestamp)>,
    animal_query: Query<(&AnimalId, &AnimalDecayRates)>,
    enclosure_query: Query<(&EnclosureId, &EnclosureDecayRates, &EnclosureStats)>,
    dynamic_enclosure_query: Query<(&EnclosureId, &DynamicObstacleTiles)>,
) {
    let Ok((timestamp_entity, timestamp)) = timestamp_query.single() else {
        return;
    };

    // If loaded entities are not hydrated yet, wait for the next frame
    if animal_query.is_empty() {
        return;
    }

    let now_unix = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    let hours_elapsed = if timestamp.value > 0 {
        let elapsed_seconds = now_unix.saturating_sub(timestamp.value) as f32;
        let hrs = elapsed_seconds / 3600.0;
        info!(
            "Loaded save from timestamp {}. Elapsed seconds: {}, offline hours: {:.3}",
            timestamp.value, elapsed_seconds, hrs
        );
        hrs
    } else {
        0.0
    };

    if hours_elapsed > 0.0 {
        for (id, decay_rates) in &animal_query {
            let hunger_decay = (decay_rates.hunger_rate * hours_elapsed).round() as u32;
            let happiness_decay = (decay_rates.happiness_rate * hours_elapsed).round() as u32;

            if hunger_decay > 0 {
                commands.trigger(WorsenStatEvent {
                    target: StatTarget::Animal {
                        id: *id,
                        stat: AnimalStat::Hunger,
                    },
                    amount: hunger_decay,
                });
            }
            if happiness_decay > 0 {
                commands.trigger(WorsenStatEvent {
                    target: StatTarget::Animal {
                        id: *id,
                        stat: AnimalStat::Happiness,
                    },
                    amount: happiness_decay,
                });
            }
        }

        for (id, decay_rates, stats) in &enclosure_query {
            let poop_count = dynamic_enclosure_query
                .iter()
                .find(|(enc_id, _)| **enc_id == *id)
                .map(|(_, tiles)| tiles.0.len())
                .unwrap_or(0);
            let cleanliness_decay = enclosure_cleanliness_decay_amount(
                stats.cleanliness,
                hours_elapsed,
                decay_rates.cleanliness_rate,
                *id,
                poop_count,
            );
            trigger_enclosure_decay(&mut commands, *id, cleanliness_decay);
        }

        let wander_steps = (hours_elapsed * OFFLINE_WANDER_STEPS_PER_HOUR).round() as u32;
        if wander_steps > 0 {
            commands.insert_resource(PendingOfflineWander {
                steps: wander_steps,
            });
        }
    }

    commands.entity(timestamp_entity).despawn();
}

fn apply_offline_wander_system(
    mut pending: ResMut<PendingOfflineWander>,
    masks: Option<Res<CollisionMasks>>,
    persisted_obstacles: Query<(&EnclosureId, &DynamicObstacleTiles)>,
    live_obstacles: Query<LiveObstacleItem<'_>>,
    mut wander_query: Query<(
        Entity,
        &AnimalEnclosure,
        &mut AnimalTilePosition,
        &AnimalBackgroundWander,
    )>,
) {
    if pending.steps == 0 {
        return;
    }

    let Some(masks) = masks else {
        return;
    };

    for (_, enclosure, _, _) in &wander_query {
        let key = CollisionMapKey::Enclosure(enclosure.0);
        if !masks.contains(key) {
            return;
        }
    }

    let steps = pending.steps;
    let mut rng = rng();
    for (entity, enclosure, mut pos, wander) in &mut wander_query {
        let key = CollisionMapKey::Enclosure(enclosure.0);
        for _ in 0..steps {
            pos.0 = random_wander_step(
                pos.0,
                wander.bounds,
                key,
                &masks,
                &persisted_obstacles,
                &live_obstacles,
                Some(entity),
                &mut rng,
            );
        }
    }

    pending.steps = 0;
}

/// Continuous decay calculation in real-time when game is running.
/// Accumulates fractional decay per-animal/per-enclosure to avoid precision issues.
pub fn tick_decay_system(
    mut commands: Commands,
    time: Res<Time>,
    mut animal_query: Query<(&AnimalId, &AnimalDecayRates, &mut AnimalDecayAccumulators)>,
    mut enclosure_query: Query<(
        &EnclosureId,
        &EnclosureDecayRates,
        &DynamicObstacleTiles,
        &mut EnclosureDecayAccumulators,
    )>,
) {
    let delta_hours = time.delta_secs() / 3600.0;

    for (id, decay_rates, mut accs) in &mut animal_query {
        accs.hunger += decay_rates.hunger_rate * delta_hours;
        accs.happiness += decay_rates.happiness_rate * delta_hours;

        if accs.hunger >= 1.0 {
            let decay_amount = accs.hunger.floor();
            accs.hunger -= decay_amount;
            commands.trigger(WorsenStatEvent {
                target: StatTarget::Animal {
                    id: *id,
                    stat: AnimalStat::Hunger,
                },
                amount: decay_amount as u32,
            });
        }
        if accs.happiness >= 1.0 {
            let decay_amount = accs.happiness.floor();
            accs.happiness -= decay_amount;
            commands.trigger(WorsenStatEvent {
                target: StatTarget::Animal {
                    id: *id,
                    stat: AnimalStat::Happiness,
                },
                amount: decay_amount as u32,
            });
        }
    }

    for (id, decay_rates, tiles, mut accs) in &mut enclosure_query {
        let effective_rate = cleanliness_decay_with_poops(
            decay_rates.cleanliness_rate,
            *id,
            tiles.0.len(),
        );
        accs.cleanliness += effective_rate * delta_hours;

        if accs.cleanliness >= 1.0 {
            let decay_amount = accs.cleanliness.floor();
            accs.cleanliness -= decay_amount;
            commands.trigger(WorsenStatEvent {
                target: StatTarget::Enclosure {
                    id: *id,
                    stat: EnclosureStat::Cleanliness,
                },
                amount: decay_amount as u32,
            });
        }
    }
}

/// Computes the arithmetic mean of all stats across active entities to find SanctuaryUpkeep score.
fn update_upkeep_system(
    animal_query: Query<&AnimalStats>,
    enclosure_query: Query<&EnclosureStats>,
    mut upkeep: ResMut<SanctuaryUpkeep>,
) {
    if animal_query.is_empty() && enclosure_query.is_empty() {
        return;
    }

    let mut total_hunger = 0;
    let mut total_happy = 0;
    let animal_count = animal_query.iter().count() as f32;

    for stats in &animal_query {
        total_hunger += stats.hunger;
        total_happy += stats.happiness;
    }

    let mut total_clean = 0;
    let enclosure_count = enclosure_query.iter().count() as f32;

    for stats in &enclosure_query {
        total_clean += stats.cleanliness;
    }

    upkeep.mean_hunger = if animal_count > 0.0 {
        (total_hunger as f32 / animal_count) / STAT_SCALE as f32
    } else {
        1.0
    };

    upkeep.mean_happiness = if animal_count > 0.0 {
        (total_happy as f32 / animal_count) / STAT_SCALE as f32
    } else {
        1.0
    };

    upkeep.mean_cleanliness = if enclosure_count > 0.0 {
        (total_clean as f32 / enclosure_count) / STAT_SCALE as f32
    } else {
        1.0
    };

    upkeep.score = (upkeep.mean_hunger + upkeep.mean_cleanliness + upkeep.mean_happiness) / 3.0;
}

/// Simple resource timer for periodic autosaves ([`AUTOSAVE_INTERVAL_SECS`])
#[derive(Resource, Deref, DerefMut)]
struct AutoSaveTimer(Timer);

impl Default for AutoSaveTimer {
    fn default() -> Self {
        Self(Timer::from_seconds(AUTOSAVE_INTERVAL_SECS, TimerMode::Repeating))
    }
}

fn save_stats_periodically_system(
    mut commands: Commands,
    time: Res<Time>,
    mut timer: ResMut<AutoSaveTimer>,
    save_path: Res<SavePath>,
    mut timestamp_query: Query<&mut SaveTimestamp>,
) {
    timer.tick(time.delta());
    if timer.just_finished() {
        let now_unix = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        match timestamp_query.single_mut() {
            Ok(mut timestamp) => {
                timestamp.value = now_unix;
            }
            Err(_) => {
                commands.spawn((
                    Name::new("Save Timestamp"),
                    SaveTimestamp { value: now_unix },
                ));
            }
        }

        info!("Autosaving stats via moonshine-save...");
        let mut save = SaveWorld::default_into_file(&save_path.0);
        save.components = bevy::world_serialization::WorldFilter::deny_all()
            .allow::<SaveTimestamp>()
            .allow::<AnimalId>()
            .allow::<AnimalStats>()
            .allow::<AnimalTilePosition>()
            .allow::<DynamicObstacleTiles>()
            .allow::<EnclosureId>()
            .allow::<EnclosureStats>();
        save.resources = bevy::world_serialization::WorldFilter::deny_all()
            .allow::<PoopWheelbarrow>();
        commands.trigger_save(save);
    }
}

/// Fast-forward simulated decay by the given number of hours.
pub fn advance_simulated_hours(
    commands: &mut Commands,
    hours: f32,
    animal_query: &Query<(&AnimalId, &AnimalDecayRates)>,
    enclosure_query: &Query<(&EnclosureId, &EnclosureDecayRates, &EnclosureStats)>,
    tiles_query: &Query<(&EnclosureId, &DynamicObstacleTiles)>,
) {
    advance_simulated_hours_queries(commands, hours, animal_query, enclosure_query, tiles_query);
}

/// Fast-forward simulated decay using a mutable world (command queue / headless dispatch).
pub fn advance_simulated_hours_world(world: &mut World, hours: f32) {
    let mut animal_query = world.query::<(&AnimalId, &AnimalDecayRates)>();
    let mut enclosure_query =
        world.query::<(&EnclosureId, &EnclosureDecayRates, &EnclosureStats)>();
    let mut tiles_query = world.query::<(&EnclosureId, &DynamicObstacleTiles)>();
    let animal_decays: Vec<(AnimalId, AnimalDecayRates)> = animal_query
        .iter(world)
        .map(|(id, rates)| (*id, rates.clone()))
        .collect();
    let enclosure_decays: Vec<(EnclosureId, EnclosureDecayRates, u32, usize)> = enclosure_query
        .iter(world)
        .map(|(id, rates, stats)| {
            let poop_count = tiles_query
                .iter(world)
                .find(|(enc_id, _)| *enc_id == id)
                .map(|(_, tiles)| tiles.0.len())
                .unwrap_or(0);
            (*id, rates.clone(), stats.cleanliness, poop_count)
        })
        .collect();

    let mut commands = world.commands();
    for (id, decay_rates) in animal_decays {
        trigger_animal_decay(&mut commands, id, &decay_rates, hours);
    }
    for (id, decay_rates, start_cleanliness, poop_count) in enclosure_decays {
        let amount = enclosure_cleanliness_decay_amount(
            start_cleanliness,
            hours,
            decay_rates.cleanliness_rate,
            id,
            poop_count,
        );
        trigger_enclosure_decay(&mut commands, id, amount);
    }
}

fn advance_simulated_hours_queries(
    commands: &mut Commands,
    hours: f32,
    animal_query: &Query<(&AnimalId, &AnimalDecayRates)>,
    enclosure_query: &Query<(&EnclosureId, &EnclosureDecayRates, &EnclosureStats)>,
    tiles_query: &Query<(&EnclosureId, &DynamicObstacleTiles)>,
) {
    info!("Advancing simulated time by {hours} hours");

    for (id, decay_rates) in animal_query {
        trigger_animal_decay(commands, *id, decay_rates, hours);
    }
    for (id, decay_rates, stats) in enclosure_query {
        let poop_count = tiles_query
            .iter()
            .find(|(enc_id, _)| *enc_id == id)
            .map(|(_, tiles)| tiles.0.len())
            .unwrap_or(0);
        let amount = enclosure_cleanliness_decay_amount(
            stats.cleanliness,
            hours,
            decay_rates.cleanliness_rate,
            *id,
            poop_count,
        );
        trigger_enclosure_decay(commands, *id, amount);
    }
}

fn trigger_animal_decay(
    commands: &mut Commands,
    id: AnimalId,
    decay_rates: &AnimalDecayRates,
    hours: f32,
) {
    let hunger_decay = (decay_rates.hunger_rate * hours).round() as u32;
    let happiness_decay = (decay_rates.happiness_rate * hours).round() as u32;

    commands.trigger(WorsenStatEvent {
        target: StatTarget::Animal {
            id,
            stat: AnimalStat::Hunger,
        },
        amount: hunger_decay,
    });
    commands.trigger(WorsenStatEvent {
        target: StatTarget::Animal {
            id,
            stat: AnimalStat::Happiness,
        },
        amount: happiness_decay,
    });
}

fn trigger_enclosure_decay(commands: &mut Commands, id: EnclosureId, amount: u32) {
    if amount == 0 {
        return;
    }
    commands.trigger(WorsenStatEvent {
        target: StatTarget::Enclosure {
            id,
            stat: EnclosureStat::Cleanliness,
        },
        amount,
    });
}

/// Debug stats control via keyboard inputs.
fn debug_stats_control_system(
    mut commands: Commands,
    input: Res<ButtonInput<KeyCode>>,
    animal_query: Query<(&AnimalId, &AnimalDecayRates)>,
    enclosure_query: Query<(&EnclosureId, &EnclosureDecayRates, &EnclosureStats)>,
    tiles_query: Query<(&EnclosureId, &DynamicObstacleTiles)>,
) {
    // Keys 1, 2, 3: care actions for Polly
    if input.just_pressed(KeyCode::Digit1) {
        commands.trigger(ImproveStatEvent {
            target: StatTarget::Animal {
                id: AnimalId::Polly,
                stat: AnimalStat::Hunger,
            },
            amount: 250,
        });
    }
    if input.just_pressed(KeyCode::Digit2) {
        commands.trigger(ImproveStatEvent {
            target: StatTarget::Animal {
                id: AnimalId::Polly,
                stat: AnimalStat::Cleanliness,
            },
            amount: 250,
        });
    }
    if input.just_pressed(KeyCode::Digit3) {
        commands.trigger(ImproveStatEvent {
            target: StatTarget::Animal {
                id: AnimalId::Polly,
                stat: AnimalStat::Happiness,
            },
            amount: 250,
        });
    }

    // Keys 4, 5, 6: care actions for Stompy
    if input.just_pressed(KeyCode::Digit4) {
        commands.trigger(ImproveStatEvent {
            target: StatTarget::Animal {
                id: AnimalId::Stompy,
                stat: AnimalStat::Hunger,
            },
            amount: 250,
        });
    }
    if input.just_pressed(KeyCode::Digit5) {
        commands.trigger(ImproveStatEvent {
            target: StatTarget::Animal {
                id: AnimalId::Stompy,
                stat: AnimalStat::Cleanliness,
            },
            amount: 250,
        });
    }
    if input.just_pressed(KeyCode::Digit6) {
        commands.trigger(ImproveStatEvent {
            target: StatTarget::Animal {
                id: AnimalId::Stompy,
                stat: AnimalStat::Happiness,
            },
            amount: 250,
        });
    }

    // Keys 7, 8, 9: care actions for Georgie
    if input.just_pressed(KeyCode::Digit7) {
        commands.trigger(ImproveStatEvent {
            target: StatTarget::Animal {
                id: AnimalId::Georgie,
                stat: AnimalStat::Hunger,
            },
            amount: 250,
        });
    }
    if input.just_pressed(KeyCode::Digit8) {
        commands.trigger(ImproveStatEvent {
            target: StatTarget::Animal {
                id: AnimalId::Georgie,
                stat: AnimalStat::Cleanliness,
            },
            amount: 250,
        });
    }
    if input.just_pressed(KeyCode::Digit9) {
        commands.trigger(ImproveStatEvent {
            target: StatTarget::Animal {
                id: AnimalId::Georgie,
                stat: AnimalStat::Happiness,
            },
            amount: 250,
        });
    }

    // Keys 0, I, O: care actions for Siren
    if input.just_pressed(KeyCode::Digit0) {
        commands.trigger(ImproveStatEvent {
            target: StatTarget::Animal {
                id: AnimalId::Siren,
                stat: AnimalStat::Hunger,
            },
            amount: 250,
        });
    }
    if input.just_pressed(KeyCode::KeyI) {
        commands.trigger(ImproveStatEvent {
            target: StatTarget::Animal {
                id: AnimalId::Siren,
                stat: AnimalStat::Cleanliness,
            },
            amount: 250,
        });
    }
    if input.just_pressed(KeyCode::KeyO) {
        commands.trigger(ImproveStatEvent {
            target: StatTarget::Animal {
                id: AnimalId::Siren,
                stat: AnimalStat::Happiness,
            },
            amount: 250,
        });
    }

    // Keys U, J, Y: care actions for Push Pop
    if input.just_pressed(KeyCode::KeyU) {
        commands.trigger(ImproveStatEvent {
            target: StatTarget::Animal {
                id: AnimalId::PushPop,
                stat: AnimalStat::Hunger,
            },
            amount: 250,
        });
    }
    if input.just_pressed(KeyCode::KeyJ) {
        commands.trigger(ImproveStatEvent {
            target: StatTarget::Animal {
                id: AnimalId::PushPop,
                stat: AnimalStat::Cleanliness,
            },
            amount: 250,
        });
    }
    if input.just_pressed(KeyCode::KeyY) {
        commands.trigger(ImproveStatEvent {
            target: StatTarget::Animal {
                id: AnimalId::PushPop,
                stat: AnimalStat::Happiness,
            },
            amount: 250,
        });
    }

    // Key -: instantly worsen all stats by 100 (10%) for testing (supports Minus, NumpadSubtract, or KeyM)
    if input.just_pressed(KeyCode::Minus)
        || input.just_pressed(KeyCode::NumpadSubtract)
        || input.just_pressed(KeyCode::KeyM)
    {
        info!("Debug: Instantly worsening all stats for all animals!");
        for (id, _) in &animal_query {
            commands.trigger(WorsenStatEvent {
                target: StatTarget::Animal {
                    id: *id,
                    stat: AnimalStat::Hunger,
                },
                amount: 100,
            });
            commands.trigger(WorsenStatEvent {
                target: StatTarget::Animal {
                    id: *id,
                    stat: AnimalStat::Happiness,
                },
                amount: 100,
            });
        }
        for (id, _, _) in &enclosure_query {
            commands.trigger(WorsenStatEvent {
                target: StatTarget::Enclosure {
                    id: *id,
                    stat: EnclosureStat::Cleanliness,
                },
                amount: 100,
            });
        }
    }

    // Key =: fast forward time by 4 hours for testing (supports Equal, NumpadAdd, or KeyL)
    if input.just_pressed(KeyCode::Equal)
        || input.just_pressed(KeyCode::NumpadAdd)
        || input.just_pressed(KeyCode::KeyL)
    {
        advance_simulated_hours_queries(
            &mut commands,
            4.0,
            &animal_query,
            &enclosure_query,
            &tiles_query,
        );
    }
}

#[derive(Resource, Deref, DerefMut)]
struct DebugLogTimer(Timer);

impl Default for DebugLogTimer {
    fn default() -> Self {
        Self(Timer::from_seconds(2.0, TimerMode::Repeating))
    }
}

fn debug_log_stats_system(
    time: Res<Time>,
    mut timer: ResMut<DebugLogTimer>,
    animal_query: Query<(&AnimalId, &AnimalName, &AnimalStats)>,
    enclosure_query: Query<(&EnclosureId, &EnclosureName, &EnclosureStats)>,
) {
    timer.0.tick(time.delta());
    if timer.0.just_finished() {
        info!(
            "--- debug_log_stats_system: Animal count = {}, Enclosure count = {} ---",
            animal_query.iter().count(),
            enclosure_query.iter().count()
        );
        for (id, name, stats) in &animal_query {
            info!(
                "  - Animal {} ({}): hunger={}, happiness={}",
                name.0,
                id.as_str(),
                stats.hunger,
                stats.happiness
            );
        }
        for (id, name, stats) in &enclosure_query {
            info!(
                "  - Enclosure {} ({}): cleanliness={}",
                name.0,
                id.as_str(),
                stats.cleanliness
            );
        }
    }
}
