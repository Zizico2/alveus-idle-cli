use bevy::prelude::*;
use moonshine_save::prelude::*;
use std::path::Path;
use std::time::SystemTime;

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
#[require(Save, Unload)]
pub struct AnimalId(pub String);

#[derive(Component, Debug, Clone, Reflect)]
#[reflect(Component)]
pub struct AnimalName(pub String);

#[derive(Component, Debug, Clone, Reflect)]
#[reflect(Component)]
pub struct AnimalStats {
    pub hunger: u32,      // [0, 1000]
    pub happiness: u32,   // [0, 1000]
}

#[derive(Component, Debug, Clone, Reflect)]
#[reflect(Component)]
pub struct AnimalDecayRates {
    pub hunger_rate: f32,      // units per hour
    pub happiness_rate: f32,   // units per hour
}

#[derive(Component, Debug, Clone, Reflect, Default)]
#[reflect(Component)]
pub struct AnimalDecayAccumulators {
    pub hunger: f32,
    pub happiness: f32,
}

#[derive(Component, Debug, Clone, Reflect)]
#[reflect(Component)]
pub struct AnimalEnclosure(pub String); // Links an animal to an EnclosureId

#[derive(Component, Debug, Clone, Reflect)]
#[reflect(Component)]
#[require(Save, Unload)]
pub struct EnclosureId(pub String);

#[derive(Component, Debug, Clone, Reflect)]
#[reflect(Component)]
pub struct EnclosureName(pub String);

#[derive(Component, Debug, Clone, Reflect)]
#[reflect(Component)]
pub struct EnclosureStats {
    pub cleanliness: u32, // [0, 1000]
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
pub enum StatType {
    Hunger,
    Cleanliness,
    Happiness,
}

#[derive(Event, Debug, Clone, Reflect)]
pub struct ImproveStatEvent {
    pub animal_id: String,
    pub stat_type: StatType,
    pub amount: u32,
}

#[derive(Event, Debug, Clone, Reflect)]
pub struct WorsenStatEvent {
    pub animal_id: String,
    pub stat_type: StatType,
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

// ---------------------------------------------------------
// Hardcoded Animal Data (Sync with design/data/animals.json)
// ---------------------------------------------------------

pub struct AnimalStaticData {
    pub animal_id: &'static str,
    pub display_name: &'static str,
    pub hunger_decay_rate: f32,
    pub happiness_decay_rate: f32,
}

/// WARNING: The static array `ANIMALS_DATA` is hardcoded to avoid runtime JSON parsing overhead
/// and ensure fast initialization. It MUST be kept in sync with the canonical config file:
/// [animals.json](file:///home/bernardo/Projects/alveus-idle-cli/design/data/animals.json) (or `design/data/animals.json`).
///
/// If any species, display names, or decay rates are modified in `animals.json`, the corresponding
/// values in this array MUST be updated manually to match.
///
/// Future developers (humans and AI agents) should always consult both files when updating
/// animal attributes or adding new animals to the sanctuary.
pub const ANIMALS_DATA: &[AnimalStaticData] = &[
    AnimalStaticData {
        animal_id: "polly",
        display_name: "Polly",
        hunger_decay_rate: 0.04,
        happiness_decay_rate: 0.05,
    },
    AnimalStaticData {
        animal_id: "stompy",
        display_name: "Stompy",
        hunger_decay_rate: 0.04,
        happiness_decay_rate: 0.05,
    },
    AnimalStaticData {
        animal_id: "georgie",
        display_name: "Georgie",
        hunger_decay_rate: 0.04,
        happiness_decay_rate: 0.05,
    },
    AnimalStaticData {
        animal_id: "siren",
        display_name: "Siren",
        hunger_decay_rate: 0.04,
        happiness_decay_rate: 0.05,
    },
];

// ---------------------------------------------------------
// Plugin
// ---------------------------------------------------------

#[derive(Resource)]
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
            .register_type::<AnimalName>()
            .register_type::<AnimalStats>()
            .register_type::<AnimalDecayRates>()
            .register_type::<AnimalDecayAccumulators>()
            .register_type::<AnimalEnclosure>()
            .register_type::<EnclosureId>()
            .register_type::<EnclosureName>()
            .register_type::<EnclosureStats>()
            .register_type::<EnclosureDecayRates>()
            .register_type::<EnclosureDecayAccumulators>()
            .register_type::<SanctuaryUpkeep>()
            .init_resource::<SanctuaryUpkeep>()
            .add_observer(save_on_default_event)
            .add_observer(load_on_default_event)
            .add_observer(hydrate_loaded_stats_observer)
            // Register decoupled observers
            .add_observer(improve_stat_observer)
            .add_observer(worsen_stat_observer)
            // Startup / Initialization when entering gameplay
            .add_systems(OnEnter(crate::screens::Screen::Gameplay), init_stats_system);

        // Update systems run when player is actively playing
        app.add_systems(
            Update,
            (
                tick_decay_system,
                update_upkeep_system,
                debug_stats_control_system,
                save_stats_periodically_system,
                debug_log_stats_system,
                apply_offline_decay_system,
            )
                .run_if(in_gameplay_or_room),
        );
    }
}

// ---------------------------------------------------------
// Run Conditions & Helpers
// ---------------------------------------------------------

fn in_gameplay_or_room(screen_state: Res<State<crate::screens::Screen>>) -> bool {
    matches!(
        screen_state.get(),
        crate::screens::Screen::Gameplay | crate::screens::Screen::InRoom(_)
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
    info!("improve_stat_observer triggered for animal/target '{}', stat '{:?}', amount {}", event.animal_id, event.stat_type, event.amount);

    match event.stat_type {
        StatType::Hunger => {
            let mut found = false;
            for (id, name, mut stats, _) in &mut animal_query {
                if id.0 == event.animal_id {
                    found = true;
                    let prev = stats.hunger;
                    stats.hunger = stats.hunger.saturating_add(event.amount).min(1000);
                    info!("Improved hunger for {} ({}): {} -> {}", name.0, id.0, prev, stats.hunger);
                }
            }
            if !found {
                warn!("ImproveStatEvent: Animal id '{}' not found in world", event.animal_id);
            }
        }
        StatType::Happiness => {
            let mut found = false;
            for (id, name, mut stats, _) in &mut animal_query {
                if id.0 == event.animal_id {
                    found = true;
                    let prev = stats.happiness;
                    stats.happiness = stats.happiness.saturating_add(event.amount).min(1000);
                    info!("Improved happiness for {} ({}): {} -> {}", name.0, id.0, prev, stats.happiness);
                }
            }
            if !found {
                warn!("ImproveStatEvent: Animal id '{}' not found in world", event.animal_id);
            }
        }
        StatType::Cleanliness => {
            let mut target_enclosure_id = event.animal_id.clone();

            // If targeted by animal_id, find the animal's enclosure
            for (id, _, _, animal_enc) in &animal_query {
                if id.0 == event.animal_id {
                    target_enclosure_id = animal_enc.0.clone();
                    break;
                }
            }

            let mut found = false;
            for (enc_id, enc_name, mut enc_stats) in &mut enclosure_query {
                if enc_id.0 == target_enclosure_id {
                    found = true;
                    let prev = enc_stats.cleanliness;
                    enc_stats.cleanliness = enc_stats.cleanliness.saturating_add(event.amount).min(1000);
                    info!("Improved cleanliness for Enclosure {} ({}): {} -> {}", enc_name.0, enc_id.0, prev, enc_stats.cleanliness);
                }
            }

            if !found {
                warn!("ImproveStatEvent: Enclosure id '{}' not found in world", target_enclosure_id);
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
    info!("worsen_stat_observer triggered for animal/target '{}', stat '{:?}', amount {}", event.animal_id, event.stat_type, event.amount);

    match event.stat_type {
        StatType::Hunger => {
            let mut found = false;
            for (id, name, mut stats, _) in &mut animal_query {
                if id.0 == event.animal_id {
                    found = true;
                    let prev = stats.hunger;
                    stats.hunger = stats.hunger.saturating_sub(event.amount);
                    info!("Worsened hunger for {} ({}): {} -> {}", name.0, id.0, prev, stats.hunger);
                }
            }
            if !found {
                warn!("WorsenStatEvent: Animal id '{}' not found in world", event.animal_id);
            }
        }
        StatType::Happiness => {
            let mut found = false;
            for (id, name, mut stats, _) in &mut animal_query {
                if id.0 == event.animal_id {
                    found = true;
                    let prev = stats.happiness;
                    stats.happiness = stats.happiness.saturating_sub(event.amount);
                    info!("Worsened happiness for {} ({}): {} -> {}", name.0, id.0, prev, stats.happiness);
                }
            }
            if !found {
                warn!("WorsenStatEvent: Animal id '{}' not found in world", event.animal_id);
            }
        }
        StatType::Cleanliness => {
            let mut target_enclosure_id = event.animal_id.clone();

            // If targeted by animal_id, find the animal's enclosure
            for (id, _, _, animal_enc) in &animal_query {
                if id.0 == event.animal_id {
                    target_enclosure_id = animal_enc.0.clone();
                    break;
                }
            }

            let mut found = false;
            for (enc_id, enc_name, mut enc_stats) in &mut enclosure_query {
                if enc_id.0 == target_enclosure_id {
                    found = true;
                    let prev = enc_stats.cleanliness;
                    enc_stats.cleanliness = enc_stats.cleanliness.saturating_sub(event.amount);
                    info!("Worsened cleanliness for Enclosure {} ({}): {} -> {}", enc_name.0, enc_id.0, prev, enc_stats.cleanliness);
                }
            }

            if !found {
                warn!("WorsenStatEvent: Enclosure id '{}' not found in world", target_enclosure_id);
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
            if let ron::Value::String(s) = key {
                if s == "resources" || s == "entities" {
                    return true;
                }
            }
        }
    }
    false
}

/// Initialize animal stats on entering gameplay screen, checking save data.
fn init_stats_system(
    mut commands: Commands,
    query: Query<&AnimalId>,
    save_path: Res<SavePath>,
) {
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
    let enclosure_mappings = [
        ("nutrition_house_playpen", "Nutrition House Playpen", 0.03 * 1000.0),
        ("pasture", "Pasture Grassland", 0.03 * 1000.0),
        ("reptile_enclosure", "Reptile Enclosure", 0.03 * 1000.0),
    ];

    for &(enc_id, enc_name, cleanliness_rate) in &enclosure_mappings {
        commands.spawn((
            Name::new(format!("Enclosure Stats - {}", enc_name)),
            EnclosureId(enc_id.to_string()),
            EnclosureName(enc_name.to_string()),
            EnclosureStats { cleanliness: 1000 },
            EnclosureDecayRates { cleanliness_rate },
            EnclosureDecayAccumulators::default(),
        ));
    }

    for animal in ANIMALS_DATA {
        let animal_id = animal.animal_id.to_string();
        let display_name = animal.display_name.to_string();

        let decay_rates = AnimalDecayRates {
            hunger_rate: animal.hunger_decay_rate * 1000.0,
            happiness_rate: animal.happiness_decay_rate * 1000.0,
        };

        let enc_id = match animal.animal_id {
            "polly" => "nutrition_house_playpen",
            "stompy" => "pasture",
            "georgie" => "reptile_enclosure",
            "siren" => "reptile_enclosure",
            _ => "unknown_enclosure",
        };

        commands.spawn((
            Name::new(format!("Persistent Stats - {}", display_name)),
            AnimalId(animal_id),
            AnimalName(display_name),
            AnimalEnclosure(enc_id.to_string()),
            AnimalStats {
                hunger: 1000,
                happiness: 1000,
            },
            decay_rates,
            AnimalDecayAccumulators::default(),
        ));
    }
}

fn hydrate_loaded_stats_observer(
    _trigger: On<Loaded>,
    mut commands: Commands,
    animal_query: Query<(Entity, &AnimalId)>,
    enclosure_query: Query<(Entity, &EnclosureId)>,
) {
    info!("Hydrating loaded stats with static config...");

    let enclosure_mappings = [
        ("nutrition_house_playpen", "Nutrition House Playpen", 0.03 * 1000.0),
        ("pasture", "Pasture Grassland", 0.03 * 1000.0),
        ("reptile_enclosure", "Reptile Enclosure", 0.03 * 1000.0),
    ];

    for (entity, id) in &animal_query {
        if let Some(animal) = ANIMALS_DATA.iter().find(|a| a.animal_id == id.0) {
            let decay_rates = AnimalDecayRates {
                hunger_rate: animal.hunger_decay_rate * 1000.0,
                happiness_rate: animal.happiness_decay_rate * 1000.0,
            };

            let enc_id = match id.0.as_str() {
                "polly" => "nutrition_house_playpen",
                "stompy" => "pasture",
                "georgie" => "reptile_enclosure",
                "siren" => "reptile_enclosure",
                _ => "unknown_enclosure",
            };

            commands.entity(entity).insert((
                Name::new(format!("Persistent Stats - {}", animal.display_name)),
                AnimalName(animal.display_name.to_string()),
                AnimalEnclosure(enc_id.to_string()),
                decay_rates,
                AnimalDecayAccumulators::default(),
            ));
        }
    }

    for (entity, id) in &enclosure_query {
        if let Some(&(_, enc_name, cleanliness_rate)) = enclosure_mappings.iter().find(|(mapped_id, _, _)| *mapped_id == id.0) {
            commands.entity(entity).insert((
                Name::new(format!("Enclosure Stats - {}", enc_name)),
                EnclosureName(enc_name.to_string()),
                EnclosureDecayRates { cleanliness_rate },
                EnclosureDecayAccumulators::default(),
            ));
        }
    }
}

fn apply_offline_decay_system(
    mut commands: Commands,
    timestamp_query: Query<(Entity, &SaveTimestamp)>,
    animal_query: Query<(&AnimalId, &AnimalDecayRates)>,
    enclosure_query: Query<(&EnclosureId, &EnclosureDecayRates)>,
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
                    animal_id: id.0.clone(),
                    stat_type: StatType::Hunger,
                    amount: hunger_decay,
                });
            }
            if happiness_decay > 0 {
                commands.trigger(WorsenStatEvent {
                    animal_id: id.0.clone(),
                    stat_type: StatType::Happiness,
                    amount: happiness_decay,
                });
            }
        }

        for (id, decay_rates) in &enclosure_query {
            let cleanliness_decay = (decay_rates.cleanliness_rate * hours_elapsed).round() as u32;
            if cleanliness_decay > 0 {
                commands.trigger(WorsenStatEvent {
                    animal_id: id.0.clone(),
                    stat_type: StatType::Cleanliness,
                    amount: cleanliness_decay,
                });
            }
        }
    }

    commands.entity(timestamp_entity).despawn();
}

/// Continuous decay calculation in real-time when game is running.
/// Accumulates fractional decay per-animal/per-enclosure to avoid precision issues.
pub fn tick_decay_system(
    mut commands: Commands,
    time: Res<Time>,
    mut animal_query: Query<(&AnimalId, &AnimalDecayRates, &mut AnimalDecayAccumulators)>,
    mut enclosure_query: Query<(&EnclosureId, &EnclosureDecayRates, &mut EnclosureDecayAccumulators)>,
) {
    let delta_hours = time.delta_secs() / 3600.0;

    for (id, decay_rates, mut accs) in &mut animal_query {
        accs.hunger += decay_rates.hunger_rate * delta_hours;
        accs.happiness += decay_rates.happiness_rate * delta_hours;

        if accs.hunger >= 1.0 {
            let decay_amount = accs.hunger.floor();
            accs.hunger -= decay_amount;
            commands.trigger(WorsenStatEvent {
                animal_id: id.0.clone(),
                stat_type: StatType::Hunger,
                amount: decay_amount as u32,
            });
        }
        if accs.happiness >= 1.0 {
            let decay_amount = accs.happiness.floor();
            accs.happiness -= decay_amount;
            commands.trigger(WorsenStatEvent {
                animal_id: id.0.clone(),
                stat_type: StatType::Happiness,
                amount: decay_amount as u32,
            });
        }
    }

    for (id, decay_rates, mut accs) in &mut enclosure_query {
        accs.cleanliness += decay_rates.cleanliness_rate * delta_hours;

        if accs.cleanliness >= 1.0 {
            let decay_amount = accs.cleanliness.floor();
            accs.cleanliness -= decay_amount;
            commands.trigger(WorsenStatEvent {
                animal_id: id.0.clone(),
                stat_type: StatType::Cleanliness,
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
        (total_hunger as f32 / animal_count) / 1000.0
    } else {
        1.0
    };

    upkeep.mean_happiness = if animal_count > 0.0 {
        (total_happy as f32 / animal_count) / 1000.0
    } else {
        1.0
    };

    upkeep.mean_cleanliness = if enclosure_count > 0.0 {
        (total_clean as f32 / enclosure_count) / 1000.0
    } else {
        1.0
    };

    upkeep.score = (upkeep.mean_hunger + upkeep.mean_cleanliness + upkeep.mean_happiness) / 3.0;
}

/// Simple resource timer for periodic autosaves (every 5 seconds)
#[derive(Resource, Deref, DerefMut)]
struct AutoSaveTimer(Timer);

impl Default for AutoSaveTimer {
    fn default() -> Self {
        Self(Timer::from_seconds(5.0, TimerMode::Repeating))
    }
}

fn save_stats_periodically_system(
    mut commands: Commands,
    time: Res<Time>,
    timer_opt: Option<ResMut<AutoSaveTimer>>,
    save_path: Res<SavePath>,
    mut timestamp_query: Query<&mut SaveTimestamp>,
) {
    let mut timer = match timer_opt {
        Some(t) => t,
        None => {
            commands.insert_resource(AutoSaveTimer::default());
            return;
        }
    };

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
        save.components = bevy::scene::SceneFilter::deny_all()
            .allow::<SaveTimestamp>()
            .allow::<AnimalId>()
            .allow::<AnimalStats>()
            .allow::<EnclosureId>()
            .allow::<EnclosureStats>();
        commands.trigger_save(save);
    }
}

/// Debug stats control via keyboard inputs.
fn debug_stats_control_system(
    mut commands: Commands,
    input: Res<ButtonInput<KeyCode>>,
    animal_query: Query<(&AnimalId, &AnimalDecayRates)>,
    enclosure_query: Query<(&EnclosureId, &EnclosureDecayRates)>,
) {
    // Keys 1, 2, 3: care actions for Polly
    if input.just_pressed(KeyCode::Digit1) {
        commands.trigger(ImproveStatEvent {
            animal_id: "polly".to_string(),
            stat_type: StatType::Hunger,
            amount: 250,
        });
    }
    if input.just_pressed(KeyCode::Digit2) {
        commands.trigger(ImproveStatEvent {
            animal_id: "polly".to_string(),
            stat_type: StatType::Cleanliness,
            amount: 250,
        });
    }
    if input.just_pressed(KeyCode::Digit3) {
        commands.trigger(ImproveStatEvent {
            animal_id: "polly".to_string(),
            stat_type: StatType::Happiness,
            amount: 250,
        });
    }

    // Keys 4, 5, 6: care actions for Stompy
    if input.just_pressed(KeyCode::Digit4) {
        commands.trigger(ImproveStatEvent {
            animal_id: "stompy".to_string(),
            stat_type: StatType::Hunger,
            amount: 250,
        });
    }
    if input.just_pressed(KeyCode::Digit5) {
        commands.trigger(ImproveStatEvent {
            animal_id: "stompy".to_string(),
            stat_type: StatType::Cleanliness,
            amount: 250,
        });
    }
    if input.just_pressed(KeyCode::Digit6) {
        commands.trigger(ImproveStatEvent {
            animal_id: "stompy".to_string(),
            stat_type: StatType::Happiness,
            amount: 250,
        });
    }

    // Keys 7, 8, 9: care actions for Georgie
    if input.just_pressed(KeyCode::Digit7) {
        commands.trigger(ImproveStatEvent {
            animal_id: "georgie".to_string(),
            stat_type: StatType::Hunger,
            amount: 250,
        });
    }
    if input.just_pressed(KeyCode::Digit8) {
        commands.trigger(ImproveStatEvent {
            animal_id: "georgie".to_string(),
            stat_type: StatType::Cleanliness,
            amount: 250,
        });
    }
    if input.just_pressed(KeyCode::Digit9) {
        commands.trigger(ImproveStatEvent {
            animal_id: "georgie".to_string(),
            stat_type: StatType::Happiness,
            amount: 250,
        });
    }

    // Keys 0, I, O: care actions for Siren
    if input.just_pressed(KeyCode::Digit0) {
        commands.trigger(ImproveStatEvent {
            animal_id: "siren".to_string(),
            stat_type: StatType::Hunger,
            amount: 250,
        });
    }
    if input.just_pressed(KeyCode::KeyI) {
        commands.trigger(ImproveStatEvent {
            animal_id: "siren".to_string(),
            stat_type: StatType::Cleanliness,
            amount: 250,
        });
    }
    if input.just_pressed(KeyCode::KeyO) {
        commands.trigger(ImproveStatEvent {
            animal_id: "siren".to_string(),
            stat_type: StatType::Happiness,
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
                animal_id: id.0.clone(),
                stat_type: StatType::Hunger,
                amount: 100,
            });
            commands.trigger(WorsenStatEvent {
                animal_id: id.0.clone(),
                stat_type: StatType::Happiness,
                amount: 100,
            });
        }
        for (id, _) in &enclosure_query {
            commands.trigger(WorsenStatEvent {
                animal_id: id.0.clone(),
                stat_type: StatType::Cleanliness,
                amount: 100,
            });
        }
    }

    // Key =: fast forward time by 4 hours for testing (supports Equal, NumpadAdd, or KeyL)
    if input.just_pressed(KeyCode::Equal)
        || input.just_pressed(KeyCode::NumpadAdd)
        || input.just_pressed(KeyCode::KeyL)
    {
        info!("Debug: Fast-forwarding time by 4 hours!");
        for (id, decay_rates) in &animal_query {
            let hunger_decay = (decay_rates.hunger_rate * 4.0).round() as u32;
            let happiness_decay = (decay_rates.happiness_rate * 4.0).round() as u32;

            commands.trigger(WorsenStatEvent {
                animal_id: id.0.clone(),
                stat_type: StatType::Hunger,
                amount: hunger_decay,
            });
            commands.trigger(WorsenStatEvent {
                animal_id: id.0.clone(),
                stat_type: StatType::Happiness,
                amount: happiness_decay,
            });
        }
        for (id, decay_rates) in &enclosure_query {
            let cleanliness_decay = (decay_rates.cleanliness_rate * 4.0).round() as u32;

            commands.trigger(WorsenStatEvent {
                animal_id: id.0.clone(),
                stat_type: StatType::Cleanliness,
                amount: cleanliness_decay,
            });
        }
    }
}

#[derive(Resource)]
struct DebugLogTimer(Timer);

fn debug_log_stats_system(
    time: Res<Time>,
    timer_opt: Option<ResMut<DebugLogTimer>>,
    mut commands: Commands,
    animal_query: Query<(&AnimalId, &AnimalName, &AnimalStats)>,
    enclosure_query: Query<(&EnclosureId, &EnclosureName, &EnclosureStats)>,
) {
    let mut timer = match timer_opt {
        Some(t) => t,
        None => {
            commands.insert_resource(DebugLogTimer(Timer::from_seconds(2.0, TimerMode::Repeating)));
            return;
        }
    };
    timer.0.tick(time.delta());
    if timer.0.just_finished() {
        info!("--- debug_log_stats_system: Animal count = {}, Enclosure count = {} ---", animal_query.iter().count(), enclosure_query.iter().count());
        for (id, name, stats) in &animal_query {
            info!("  - Animal {} ({}): hunger={}, happiness={}", name.0, id.0, stats.hunger, stats.happiness);
        }
        for (id, name, stats) in &enclosure_query {
            info!("  - Enclosure {} ({}): cleanliness={}", name.0, id.0, stats.cleanliness);
        }
    }
}

