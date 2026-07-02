//! Poop spawning, wheelbarrow pickup, and compost-dump cleaning loop.

use bevy::prelude::*;

use crate::collision::DynamicObstacleTiles;
use crate::components::TilePosition;
use crate::interaction::{Interactable, LastPickupMessage};
use crate::screens::InRoom;
use crate::stats::{
    EnclosureStat, ImproveStatEvent, StatTarget,
};
use alveus_types::EnclosureId;

// Poop config data lives in `alveus-configs`; re-export so
// `crate::cleaning::*` paths keep resolving for the decay math and tests below.
pub use alveus_configs::{poop_config_for, PoopConfig, WHEELBARROW_CAPACITY};

pub fn room_for_enclosure(id: EnclosureId) -> Option<InRoom> {
    match id {
        EnclosureId::PushPopEnclosure => Some(InRoom::PushPopEnclosure),
        EnclosureId::NutritionHousePlaypen => Some(InRoom::NutritionHouse),
        _ => None,
    }
}

fn active_poop_config_for(id: EnclosureId) -> Option<&'static PoopConfig> {
    match id {
        EnclosureId::PushPopEnclosure => Some(poop_config_for(id)),
        EnclosureId::NutritionHousePlaypen
        | EnclosureId::Pasture
        | EnclosureId::ReptileEnclosure => None,
    }
}

/// How many poops should be on the floor given current enclosure cleanliness.
pub fn target_poop_count(cleanliness: u32, thresholds: &[u32]) -> u32 {
    thresholds
        .iter()
        .filter(|&&threshold| cleanliness <= threshold)
        .count() as u32
}

pub fn cleanliness_decay_with_poops(
    base_rate: f32,
    enclosure_id: EnclosureId,
    poop_count: usize,
) -> f32 {
    base_rate
        + active_poop_config_for(enclosure_id)
            .map(|c| c.poop_decay_rate * poop_count as f32)
            .unwrap_or(0.0)
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
    starting_poop_count: usize,
) -> u32 {
    if hours <= 0.0 {
        return 0;
    }
    if let Some(config) = active_poop_config_for(enclosure_id) {
        start.saturating_sub(cleanliness_after_threshold_decay(
            start,
            hours,
            base_rate,
            config,
        ))
    } else {
        let rate = cleanliness_decay_with_poops(base_rate, enclosure_id, starting_poop_count);
        (rate * hours).round() as u32
    }
}

// ---------------------------------------------------------
// Components / resources / events
// ---------------------------------------------------------

/// Runtime-spawned poop pile the player can pick up (not Tiled-authored).
#[derive(Component, Debug, Clone, Copy, Reflect, Default)]
#[reflect(Component, Default)]
#[require(Interactable)]
pub struct PoopPile {
    pub enclosure_id: EnclosureId,
}

/// Compost bin / dump station on the overview map (or future shared yard). Not tied to one enclosure.
#[derive(Component, Debug, Clone, Reflect)]
#[reflect(Component)]
#[require(Interactable)]
pub struct PoopDump {
    pub prompt: String,
}

#[derive(Resource, Debug, Clone, Default, Reflect)]
#[reflect(Resource)]
pub struct PoopWheelbarrow {
    /// One entry per poop picked up, in pickup order (max [`WHEELBARROW_CAPACITY`]).
    pub poops: Vec<EnclosureId>,
}

impl PoopWheelbarrow {
    pub fn count(&self) -> u8 {
        self.poops.len().min(u8::MAX as usize) as u8
    }
}

#[derive(Event, Debug, Clone, Copy, Reflect)]
#[reflect(Event)]
pub struct PoopPickedUpEvent {
    pub entity: Entity,
    pub enclosure_id: EnclosureId,
    pub tile: TilePosition,
}

#[derive(Event, Debug, Clone, Reflect)]
#[reflect(Event)]
pub struct PoopDumpedEvent {
    pub poops: Vec<EnclosureId>,
}

// ---------------------------------------------------------
// Helpers
// ---------------------------------------------------------

pub fn try_pickup_poop(
    wheelbarrow: &mut PoopWheelbarrow,
    enclosure_id: EnclosureId,
) -> Result<(), &'static str> {
    if wheelbarrow.poops.len() >= WHEELBARROW_CAPACITY as usize {
        return Err("Wheelbarrow is full — empty it at the compost bin");
    }
    wheelbarrow.poops.push(enclosure_id);
    Ok(())
}

pub fn try_dump_poop(wheelbarrow: &PoopWheelbarrow) -> Result<Vec<EnclosureId>, &'static str> {
    if wheelbarrow.poops.is_empty() {
        return Err("Wheelbarrow is empty");
    }
    Ok(wheelbarrow.poops.clone())
}

// ---------------------------------------------------------
// Plugin
// ---------------------------------------------------------

pub struct CleaningPlugin;

impl Plugin for CleaningPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<PoopPile>()
            .register_type::<PoopDump>()
            .register_type::<PoopWheelbarrow>()
            .register_type::<PoopPickedUpEvent>()
            .register_type::<PoopDumpedEvent>()
            .init_resource::<PoopWheelbarrow>()
            .add_observer(apply_poop_pickup)
            .add_observer(apply_poop_dump);
    }
}

pub fn apply_poop_pickup(
    trigger: On<PoopPickedUpEvent>,
    wheelbarrow: Res<PoopWheelbarrow>,
    mut enclosure_query: Query<(&EnclosureId, &mut DynamicObstacleTiles)>,
    mut commands: Commands,
) {
    let event = trigger.event();
    if let Some((_, mut tiles)) = enclosure_query
        .iter_mut()
        .find(|(id, _)| **id == event.enclosure_id)
    {
        tiles.remove(event.tile);
    }

    commands.entity(event.entity).despawn();

    if let Some(config) = active_poop_config_for(event.enclosure_id) {
        commands.trigger(ImproveStatEvent {
            target: StatTarget::Enclosure {
                id: event.enclosure_id,
                stat: EnclosureStat::Cleanliness,
            },
            amount: config.cleanliness_restore_per_poop,
        });
    }

    commands.insert_resource(LastPickupMessage {
        text: Some(format!(
            "Picked up poop ({}/{})",
            wheelbarrow.count(),
            WHEELBARROW_CAPACITY
        )),
        timer: Timer::from_seconds(2.5, TimerMode::Once),
    });
}

pub fn apply_poop_dump(
    trigger: On<PoopDumpedEvent>,
    mut wheelbarrow: ResMut<PoopWheelbarrow>,
    mut commands: Commands,
) {
    let _event = trigger.event();
    wheelbarrow.poops.clear();
    commands.insert_resource(LastPickupMessage {
        text: Some("Emptied wheelbarrow".to_string()),
        timer: Timer::from_seconds(2.5, TimerMode::Once),
    });
}
