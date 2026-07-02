//! Poop spawning, wheelbarrow pickup, and compost-dump cleaning loop.

use std::borrow::Borrow;

use bevy::prelude::*;
use rand::prelude::*;

use crate::collision::{
    DynamicObstacleTiles, LiveObstacleItem, CollisionMapKey, CollisionMasks,
    enclosure_for_room, is_walkable_with_dynamic,
};
use crate::components::{CurrentTilePosition, DynamicObstacle, InEnclosure, TilePosition};
use crate::demo::level::TILE_SIZE;
use crate::demo::player::Player;
use crate::interaction::{Interactable, LastPickupMessage};
use crate::screens::{InRoom, Screen};
use crate::stats::{EnclosureStat, EnclosureStats, ImproveStatEvent, StatTarget};
use crate::AppSystems;
use alveus_types::EnclosureId;

// Poop config data lives in `alveus-configs`; re-export so
// `crate::cleaning::*` paths keep resolving for the decay math and tests below.
pub use alveus_configs::{PoopConfig, WHEELBARROW_CAPACITY, poop_config_for};

pub fn room_for_enclosure(id: EnclosureId) -> InRoom {
    match id {
        EnclosureId::PushPopEnclosure => InRoom::PushPopEnclosure,
        EnclosureId::NutritionHousePlaypen => InRoom::NutritionHouse,
        EnclosureId::Pasture => InRoom::Pasture,
        EnclosureId::ReptileEnclosure => InRoom::ReptileEnclosure,
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

pub fn pick_random_poop_tile_with_blocked(
    config: &PoopConfig,
    key: CollisionMapKey,
    masks: &CollisionMasks,
    blocked: &[TilePosition],
    live_tiles: impl IntoIterator<Item = impl Borrow<TilePosition>>,
    exclude: Option<TilePosition>,
    rng: &mut impl Rng,
) -> Option<TilePosition> {
    let live: Vec<TilePosition> = live_tiles.into_iter().map(|t| *t.borrow()).collect();
    let mut candidates = Vec::new();
    for x in config.spawn_bounds.bottom_left.x..=config.spawn_bounds.top_right.x {
        for y in config.spawn_bounds.bottom_left.y..=config.spawn_bounds.top_right.y {
            let tile = TilePosition { x, y };
            if exclude == Some(tile) {
                continue;
            }
            if blocked.contains(&tile) {
                continue;
            }
            if is_walkable_with_dynamic(&masks, blocked.iter(), live.iter(), key, tile) {
                candidates.push(tile);
            }
        }
    }
    candidates.choose(rng).copied()
}

pub fn pick_random_poop_tile(
    enclosure_id: EnclosureId,
    config: &PoopConfig,
    key: CollisionMapKey,
    masks: &CollisionMasks,
    tiles: &DynamicObstacleTiles,
    live_obstacles: &Query<LiveObstacleItem<'_>>,
    exclude: Option<TilePosition>,
    rng: &mut impl Rng,
) -> Option<TilePosition> {
    let live: Vec<TilePosition> = live_obstacles
        .iter()
        .filter_map(|(_, _, pos, in_enc)| {
            in_enc
                .filter(|enc| enc.0 == enclosure_id)
                .map(|_| pos.0)
        })
        .collect();
    pick_random_poop_tile_with_blocked(
        config,
        key,
        masks,
        &tiles.0,
        live,
        exclude,
        rng,
    )
}

pub fn spawn_poop_entity(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<ColorMaterial>,
    tile: TilePosition,
    enclosure_id: EnclosureId,
    room: InRoom,
) {
    let mesh = meshes.add(Circle::new(10.0));
    let material = materials.add(Color::srgb(0.45, 0.28, 0.12));

    commands.spawn((
        Name::new("Poop Pile"),
        PoopPile { enclosure_id },
        DynamicObstacle,
        InEnclosure(enclosure_id),
        TilePosition {
            x: tile.x,
            y: tile.y,
        },
        CurrentTilePosition(tile),
        Mesh2d(mesh),
        MeshMaterial2d(material),
        Transform::from_xyz(
            tile.x as f32 * TILE_SIZE as f32,
            tile.y as f32 * TILE_SIZE as f32,
            0.4,
        ),
        DespawnOnExit(Screen::InRoom(room)),
    ));
}

/// Spawn visible poop entities for tiles that exist in save state but have no live entity yet.
pub fn sync_missing_poop_entities(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<ColorMaterial>,
    room: InRoom,
    enclosure_id: EnclosureId,
    tiles: &DynamicObstacleTiles,
    existing: &Query<&TilePosition, With<PoopPile>>,
) {
    let existing_tiles: std::collections::HashSet<TilePosition> =
        existing.iter().copied().collect();
    for &tile in &tiles.0 {
        if !existing_tiles.contains(&tile) {
            spawn_poop_entity(commands, meshes, materials, tile, enclosure_id, room);
        }
    }
}

/// Add floor poops until the count matches [`target_poop_count`] for the given cleanliness.
pub fn sync_threshold_poops_for_config(
    enclosure_id: EnclosureId,
    config: &PoopConfig,
    cleanliness: u32,
    masks: &CollisionMasks,
    tiles: &mut DynamicObstacleTiles,
    live_obstacles: &Query<LiveObstacleItem<'_>>,
    exclude: Option<TilePosition>,
    rng: &mut impl Rng,
) -> Vec<TilePosition> {
    let key = CollisionMapKey::Enclosure(enclosure_id);
    if !masks.contains(key) {
        return Vec::new();
    }

    let target = target_poop_count(cleanliness, config.spawn_thresholds);
    let mut spawned = Vec::new();

    while (tiles.0.len() as u32) < target {
        let Some(tile) = pick_random_poop_tile(
            enclosure_id,
            config,
            key,
            masks,
            tiles,
            live_obstacles,
            exclude,
            rng,
        ) else {
            break;
        };
        tiles.insert(tile);
        spawned.push(tile);
    }

    spawned
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
            .add_observer(apply_poop_dump)
            .add_systems(
                Update,
                sync_threshold_poop_spawn_system
                    .in_set(AppSystems::DecayCalculation)
                    .after(crate::stats::tick_decay_system)
                    .after(crate::stats::apply_offline_decay_system),
            )
            .add_systems(
                OnEnter(Screen::InRoom(InRoom::PushPopEnclosure)),
                spawn_poop_entities_on_push_pop_enter,
            );
    }
}

fn sync_threshold_poop_spawn_system(
    screen: Res<State<Screen>>,
    masks: Option<Res<CollisionMasks>>,
    player_query: Query<&CurrentTilePosition, With<Player>>,
    live_obstacles: Query<LiveObstacleItem<'_>>,
    mut enclosure_query: Query<(
        &EnclosureId,
        &EnclosureStats,
        &mut DynamicObstacleTiles,
    )>,
    mut commands: Commands,
    mut meshes: Option<ResMut<Assets<Mesh>>>,
    mut materials: Option<ResMut<Assets<ColorMaterial>>>,
) {
    let Some(masks) = masks else {
        return;
    };
    if !crate::collision::collision_ready(&masks) {
        return;
    }

    let player_tile = player_query.single().ok().map(|p| p.0);
    let mut rng = rand::rng();

    for (enclosure_id, stats, mut tiles) in enclosure_query.iter_mut() {
        let config = poop_config_for(*enclosure_id);

        let in_room = matches!(
            screen.get(),
            Screen::InRoom(room) if enclosure_for_room(*room) == *enclosure_id
        );
        let room = room_for_enclosure(*enclosure_id);

        let spawned = sync_threshold_poops_for_config(
            *enclosure_id,
            config,
            stats.cleanliness,
            &masks,
            &mut tiles,
            &live_obstacles,
            player_tile,
            &mut rng,
        );

        if in_room
            && let (Some(meshes), Some(materials)) = (meshes.as_deref_mut(), materials.as_deref_mut())
        {
            for tile in spawned {
                spawn_poop_entity(
                    &mut commands,
                    meshes,
                    materials,
                    tile,
                    *enclosure_id,
                    room,
                );
            }
        }
    }
}

fn spawn_poop_entities_on_push_pop_enter(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    enclosure_query: Query<(&EnclosureId, &DynamicObstacleTiles)>,
) {
    let Some((_, tiles)) = enclosure_query
        .iter()
        .find(|(id, _)| **id == EnclosureId::PushPopEnclosure)
    else {
        return;
    };

    for &tile in &tiles.0 {
        spawn_poop_entity(
            &mut commands,
            &mut meshes,
            &mut materials,
            tile,
            EnclosureId::PushPopEnclosure,
            InRoom::PushPopEnclosure,
        );
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

    let config = poop_config_for(event.enclosure_id);
    commands.trigger(ImproveStatEvent {
        target: StatTarget::Enclosure {
            id: event.enclosure_id,
            stat: EnclosureStat::Cleanliness,
        },
        amount: config.cleanliness_restore_per_poop,
    });

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
