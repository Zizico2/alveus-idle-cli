//! Collision: static masks from Tiled assets + save-backed dynamic blocked tiles.
//!
//! This crate also owns the Tiled map-handle resources ([`LevelAssets`],
//! [`InteriorAssets`]) since the collision masks are derived directly from them.

use std::borrow::Borrow;
use std::collections::{HashMap, HashSet};

use bevy::prelude::*;
use bevy::reflect::TypePath;
use bevy_ecs_tiled::prelude::*;
use moonshine_save::prelude::{Save, Unload};
use rand::prelude::*;
use tiled::{LayerType, PropertyValue};

use alveus_app::{InRoom, Screen};
use alveus_components::{
    CurrentTilePosition, DynamicObstacle, InEnclosure, Obstacle, PersistedDynamicObstacle,
    TilePosition,
};
use alveus_content::{
    TileBounds, adjacent_tiles, animal_default_placement, enclosure_for_animal, tile_in_bounds,
};
use alveus_types::{AnimalId, EnclosureId};

// ---------------------------------------------------------
// Level assets (Tiled map handles)
// ---------------------------------------------------------

/// Handles used by the overview world.
///
/// This resource is available immediately; callers inspect the referenced map's
/// root and dependency load states before using it.
#[derive(Resource, Clone, Reflect)]
#[reflect(Resource)]
pub struct LevelAssets {
    pub map: Handle<TiledMapAsset>,
}

impl FromWorld for LevelAssets {
    fn from_world(world: &mut World) -> Self {
        let assets = world.resource::<AssetServer>();
        Self {
            map: assets.load("maps/overview/map.tmx"),
        }
    }
}

/// Handles used by interior worlds. See [`LevelAssets`].
#[derive(Resource, Clone, Reflect)]
#[reflect(Resource)]
pub struct InteriorAssets {
    pub nutrition_house: Handle<TiledMapAsset>,
    pub push_pop_enclosure: Handle<TiledMapAsset>,
}

impl FromWorld for InteriorAssets {
    fn from_world(world: &mut World) -> Self {
        let assets = world.resource::<AssetServer>();
        Self {
            nutrition_house: assets.load("maps/interiors/nutrition_house_interior.tmx"),
            push_pop_enclosure: assets.load("maps/interiors/push_pop_enclosure_interior.tmx"),
        }
    }
}

impl InteriorAssets {
    pub fn collision_entries(&self) -> [(EnclosureId, Handle<TiledMapAsset>); 2] {
        [
            (
                EnclosureId::NutritionHousePlaypen,
                self.nutrition_house.clone(),
            ),
            (
                EnclosureId::PushPopEnclosure,
                self.push_pop_enclosure.clone(),
            ),
        ]
    }
}

// ---------------------------------------------------------
// Collision keys & masks
// ---------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Reflect)]
#[reflect(Debug, PartialEq, Hash)]
pub enum CollisionMapKey {
    Overview,
    Enclosure(EnclosureId),
}

impl CollisionMapKey {
    pub fn for_screen(screen: &Screen) -> Self {
        match screen {
            Screen::InRoom(room) => Self::Enclosure(enclosure_for_room(*room)),
            _ => Self::Overview,
        }
    }

    pub fn enclosure_id(self) -> Option<EnclosureId> {
        match self {
            Self::Overview => None,
            Self::Enclosure(id) => Some(id),
        }
    }

    /// Asset path used when requesting this map for collision masks.
    pub fn asset_path(self) -> &'static str {
        match self {
            Self::Overview => "maps/overview/map.tmx",
            Self::Enclosure(EnclosureId::NutritionHousePlaypen) => {
                "maps/interiors/nutrition_house_interior.tmx"
            }
            Self::Enclosure(EnclosureId::PushPopEnclosure) => {
                "maps/interiors/push_pop_enclosure_interior.tmx"
            }
            // Not yet shipped as collision-required interiors.
            Self::Enclosure(EnclosureId::Pasture) => "maps/interiors/pasture_interior.tmx",
            Self::Enclosure(EnclosureId::ReptileEnclosure) => {
                "maps/interiors/reptile_enclosure_interior.tmx"
            }
        }
    }
}

/// Stable reason category when a required collision map fails to load.
/// Full Bevy load-state debug details stay in logs (not reflected).
pub const COLLISION_LOAD_REASON_ROOT_ASSET_FAILED: &str = "root_asset_failed";
pub const COLLISION_LOAD_REASON_RECURSIVE_DEPENDENCY_FAILED: &str = "recursive_dependency_failed";

/// One failed required collision map, observable via BRP `world.get_resources`.
#[derive(Reflect, Debug, Clone, PartialEq, Eq)]
pub struct CollisionLoadFailure {
    pub key: CollisionMapKey,
    pub asset_path: String,
    pub reason: String,
}

/// Deduplicated required-map load failures for the current Loading attempt.
#[derive(Resource, Reflect, Debug, Clone, Default)]
#[reflect(Resource)]
pub struct CollisionLoadFailures {
    pub entries: Vec<CollisionLoadFailure>,
}

impl CollisionLoadFailures {
    pub fn clear(&mut self) {
        self.entries.clear();
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn contains_key(&self, key: CollisionMapKey) -> bool {
        self.entries.iter().any(|entry| entry.key == key)
    }

    /// Record a failure if this key is new. Returns `true` when inserted.
    pub fn record(&mut self, failure: CollisionLoadFailure) -> bool {
        if self.contains_key(failure.key) {
            return false;
        }
        self.entries.push(failure);
        true
    }

    /// Concise toast copy for the Title screen (fits the fixed 240×60 toast).
    pub fn toast_message(&self) -> String {
        match self.entries.len() {
            0 => String::new(),
            1 => format!("Could not load map ({:?}). See log.", self.entries[0].key),
            n => format!("Could not load {n} maps. See log."),
        }
    }

    /// Detailed Loading-screen copy (dev builds include key/path/reason lines).
    pub fn loading_detail_message(&self) -> String {
        if self.entries.is_empty() {
            return String::new();
        }

        let mut msg = String::from("Could not load a required map. Returning to title…");

        #[cfg(debug_assertions)]
        {
            msg.push('\n');
            for entry in self.entries.iter().take(MAX_FAILURE_DETAILS_IN_UI) {
                msg.push_str(&format!(
                    "\n- {:?}: {} ({})",
                    entry.key, entry.asset_path, entry.reason
                ));
            }
            if self.entries.len() > MAX_FAILURE_DETAILS_IN_UI {
                msg.push_str(&format!(
                    "\n…and {} more",
                    self.entries.len() - MAX_FAILURE_DETAILS_IN_UI
                ));
            }
            msg.push_str("\nCheck the asset and log for details.");
        }

        truncate_ui_message(msg)
    }
}

const MAX_FAILURE_DETAILS_IN_UI: usize = 3;
const MAX_UI_MESSAGE_CHARS: usize = 400;

fn truncate_ui_message(mut msg: String) -> String {
    if msg.chars().count() > MAX_UI_MESSAGE_CHARS {
        msg = msg
            .chars()
            .take(MAX_UI_MESSAGE_CHARS.saturating_sub(1))
            .collect();
        msg.push('…');
    }
    msg
}

pub fn enclosure_for_room(room: InRoom) -> EnclosureId {
    match room {
        InRoom::NutritionHouse => EnclosureId::NutritionHousePlaypen,
        InRoom::PushPopEnclosure => EnclosureId::PushPopEnclosure,
        InRoom::Pasture => EnclosureId::Pasture,
        InRoom::ReptileEnclosure => EnclosureId::ReptileEnclosure,
    }
}

/// Static blocked tiles derived from Tiled `obstacle` tile properties.
#[derive(Resource, Default)]
pub struct CollisionMasks {
    static_blocked: HashMap<CollisionMapKey, HashSet<TilePosition>>,
}

/// Save-backed dynamic blocked tiles for an enclosure (e.g. poop piles in Push Pop).
#[derive(Component, Debug, Clone, Reflect, Default)]
#[reflect(Component)]
#[require(Save, Unload)]
pub struct DynamicObstacleTiles(pub Vec<TilePosition>);

impl DynamicObstacleTiles {
    pub fn contains(&self, tile: TilePosition) -> bool {
        self.0.contains(&tile)
    }

    pub fn insert(&mut self, tile: TilePosition) {
        if !self.0.contains(&tile) {
            self.0.push(tile);
        }
    }

    pub fn remove(&mut self, tile: TilePosition) {
        self.0.retain(|t| *t != tile);
    }

    pub fn clear(&mut self) {
        self.0.clear();
    }
}

/// Maps required before gameplay movement / wander.
pub const REQUIRED_COLLISION_KEYS: &[CollisionMapKey] = &[
    CollisionMapKey::Overview,
    CollisionMapKey::Enclosure(EnclosureId::NutritionHousePlaypen),
    CollisionMapKey::Enclosure(EnclosureId::PushPopEnclosure),
];

impl CollisionMasks {
    pub fn contains(&self, key: CollisionMapKey) -> bool {
        self.static_blocked.contains_key(&key)
    }

    pub fn set_static_mask(&mut self, key: CollisionMapKey, blocked: HashSet<TilePosition>) {
        self.static_blocked.insert(key, blocked);
    }

    pub fn remove(&mut self, key: CollisionMapKey) {
        self.static_blocked.remove(&key);
    }

    pub fn is_statically_walkable(&self, key: CollisionMapKey, tile: TilePosition) -> bool {
        match self.static_blocked.get(&key) {
            Some(blocked) => !blocked.contains(&tile),
            None => false,
        }
    }
}

pub fn collision_ready(masks: &CollisionMasks) -> bool {
    REQUIRED_COLLISION_KEYS
        .iter()
        .all(|key| masks.contains(*key))
}

/// Live obstacle query item: entity, position, and optional enclosure scope.
pub type LiveObstacleItem<'a> = (
    Entity,
    &'a DynamicObstacle,
    &'a CurrentTilePosition,
    Option<&'a InEnclosure>,
);

pub fn live_obstacle_blocks(key: CollisionMapKey, in_enclosure: Option<&InEnclosure>) -> bool {
    match in_enclosure {
        None => true,
        Some(InEnclosure(id)) => key.enclosure_id() == Some(*id),
    }
}

pub fn is_walkable(
    static_masks: &CollisionMasks,
    persisted_obstacles: &Query<(&EnclosureId, &DynamicObstacleTiles)>,
    live_obstacles: &Query<LiveObstacleItem<'_>>,
    key: CollisionMapKey,
    tile: TilePosition,
    ignore_entity: Option<Entity>,
) -> bool {
    if !static_masks.is_statically_walkable(key, tile) {
        return false;
    }

    if let Some(enc_id) = key.enclosure_id() {
        for (id, tiles) in persisted_obstacles.iter() {
            if *id == enc_id && tiles.contains(tile) {
                return false;
            }
        }
    }

    for (entity, _, pos, in_enclosure) in live_obstacles.iter() {
        if ignore_entity == Some(entity) {
            continue;
        }
        if pos.0 == tile && live_obstacle_blocks(key, in_enclosure) {
            return false;
        }
    }

    true
}

pub fn is_walkable_with_dynamic(
    static_masks: &CollisionMasks,
    persisted_tiles: impl IntoIterator<Item = impl Borrow<TilePosition>>,
    live_tiles: impl IntoIterator<Item = impl Borrow<TilePosition>>,
    key: CollisionMapKey,
    tile: TilePosition,
) -> bool {
    if !static_masks.is_statically_walkable(key, tile) {
        return false;
    }

    for blocked in persisted_tiles {
        if *blocked.borrow() == tile {
            return false;
        }
    }

    for blocked in live_tiles {
        if *blocked.borrow() == tile {
            return false;
        }
    }

    true
}

/// Pick a spawn tile: use `preferred` when walkable, otherwise the nearest walkable tile in `bounds`.
pub fn resolve_spawn_tile(
    preferred: TilePosition,
    bounds: TileBounds,
    key: CollisionMapKey,
    static_masks: &CollisionMasks,
    persisted_obstacles: &Query<(&EnclosureId, &DynamicObstacleTiles)>,
    live_obstacles: &Query<LiveObstacleItem<'_>>,
    ignore_entity: Option<Entity>,
) -> TilePosition {
    if is_walkable(
        static_masks,
        persisted_obstacles,
        live_obstacles,
        key,
        preferred,
        ignore_entity,
    ) {
        return preferred;
    }

    let mut queue = std::collections::VecDeque::from([preferred]);
    let mut seen = HashSet::from([preferred]);

    while let Some(tile) = queue.pop_front() {
        for next in adjacent_tiles(tile) {
            if !tile_in_bounds(next, bounds) || seen.contains(&next) {
                continue;
            }
            if is_walkable(
                static_masks,
                persisted_obstacles,
                live_obstacles,
                key,
                next,
                ignore_entity,
            ) {
                return next;
            }
            seen.insert(next);
            queue.push_back(next);
        }
    }

    preferred
}

pub struct CollisionPlugin;

impl Plugin for CollisionPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CollisionMasks>()
            .init_resource::<CollisionLoadFailures>()
            .register_type::<CollisionMapKey>()
            .register_type::<CollisionLoadFailure>()
            .register_type::<CollisionLoadFailures>()
            .register_type::<DynamicObstacleTiles>()
            .register_type::<PersistedDynamicObstacle>()
            .add_systems(
                OnEnter(Screen::Gameplay),
                build_collision_masks_on_gameplay_enter,
            )
            .add_systems(
                Update,
                (
                    rebuild_collision_masks_on_asset_events,
                    sync_dynamic_obstacle_positions,
                    cleanup_dynamic_obstacle_tiles,
                ),
            )
            .add_observer(register_dynamic_obstacle_spawn);
    }
}

pub fn collision_key_for_animal(animal_id: AnimalId) -> Option<CollisionMapKey> {
    animal_default_placement(animal_id)
        .map(|_| CollisionMapKey::Enclosure(enclosure_for_animal(animal_id)))
}

pub fn walkable_neighbors(
    from: TilePosition,
    bounds: TileBounds,
    key: CollisionMapKey,
    static_masks: &CollisionMasks,
    persisted_obstacles: &Query<(&EnclosureId, &DynamicObstacleTiles)>,
    live_obstacles: &Query<LiveObstacleItem<'_>>,
    ignore_entity: Option<Entity>,
) -> Vec<TilePosition> {
    if !static_masks.contains(key) {
        return Vec::new();
    }

    adjacent_tiles(from)
        .into_iter()
        .filter(|tile| {
            tile_in_bounds(*tile, bounds)
                && is_walkable(
                    static_masks,
                    persisted_obstacles,
                    live_obstacles,
                    key,
                    *tile,
                    ignore_entity,
                )
        })
        .collect()
}

pub fn random_wander_step(
    from: TilePosition,
    bounds: TileBounds,
    key: CollisionMapKey,
    static_masks: &CollisionMasks,
    persisted_obstacles: &Query<(&EnclosureId, &DynamicObstacleTiles)>,
    live_obstacles: &Query<LiveObstacleItem<'_>>,
    ignore_entity: Option<Entity>,
    rng: &mut impl Rng,
) -> TilePosition {
    let candidates = walkable_neighbors(
        from,
        bounds,
        key,
        static_masks,
        persisted_obstacles,
        live_obstacles,
        ignore_entity,
    );
    candidates.choose(rng).copied().unwrap_or(from)
}

pub fn build_all_collision_masks(
    masks: &mut CollisionMasks,
    map_assets: &Assets<TiledMapAsset>,
    level_assets: &LevelAssets,
    interior_assets: &InteriorAssets,
) {
    let handles = std::iter::once((CollisionMapKey::Overview, level_assets.map.clone())).chain(
        interior_assets
            .collision_entries()
            .into_iter()
            .map(|(id, handle)| (CollisionMapKey::Enclosure(id), handle)),
    );

    for (key, handle) in handles {
        let Some(asset) = map_assets.get(&handle) else {
            continue;
        };

        let built = build_mask_for_asset(asset);
        info!(
            "Built collision mask for {:?} ({} static blocked tiles)",
            key,
            built.len()
        );
        masks.static_blocked.insert(key, built);
    }
}

/// Combined state of a root map and all of its recursive dependencies.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RequiredCollisionMapState {
    Pending,
    Loaded,
    Failed,
}

pub fn required_collision_map_state(
    asset_server: &AssetServer,
    handle: &Handle<TiledMapAsset>,
) -> RequiredCollisionMapState {
    use bevy::asset::{LoadState, RecursiveDependencyLoadState};

    match asset_server.get_load_states(handle) {
        Some((LoadState::Failed(_), _, _))
        | Some((_, _, RecursiveDependencyLoadState::Failed(_))) => {
            RequiredCollisionMapState::Failed
        }
        Some((LoadState::Loaded, _, RecursiveDependencyLoadState::Loaded)) => {
            RequiredCollisionMapState::Loaded
        }
        _ => RequiredCollisionMapState::Pending,
    }
}

fn collision_load_failure_reason(
    asset_server: &AssetServer,
    handle: &Handle<TiledMapAsset>,
) -> Option<&'static str> {
    use bevy::asset::{LoadState, RecursiveDependencyLoadState};

    match asset_server.get_load_states(handle) {
        Some((LoadState::Failed(_), _, _)) => Some(COLLISION_LOAD_REASON_ROOT_ASSET_FAILED),
        Some((_, _, RecursiveDependencyLoadState::Failed(_))) => {
            Some(COLLISION_LOAD_REASON_RECURSIVE_DEPENDENCY_FAILED)
        }
        _ => None,
    }
}

pub fn required_collision_maps_terminal(
    asset_server: &AssetServer,
    handles: &[(CollisionMapKey, Handle<TiledMapAsset>)],
) -> bool {
    handles.iter().all(|(_, handle)| {
        required_collision_map_state(asset_server, handle) != RequiredCollisionMapState::Pending
    })
}

/// Detect required Tiled maps that failed to load and record them once per key.
///
/// Logging is a side effect of recording a *new* failure. Deduplicates by
/// [`CollisionMapKey`]. Full Bevy load-state details go to the log; the resource
/// stores a stable reason category for BRP/UI.
pub fn record_failed_collision_map_loads(
    asset_server: &AssetServer,
    handles: &[(CollisionMapKey, Handle<TiledMapAsset>)],
    failures: &mut CollisionLoadFailures,
) {
    for &(key, ref handle) in handles {
        if failures.contains_key(key) {
            continue;
        }

        if required_collision_map_state(asset_server, handle) == RequiredCollisionMapState::Failed {
            let failure = CollisionLoadFailure {
                key,
                asset_path: key.asset_path().to_string(),
                reason: collision_load_failure_reason(asset_server, handle)
                    .expect("Failed state has a stable reason")
                    .to_string(),
            };
            if failures.record(failure) {
                let states = asset_server.get_load_states(handle);
                error!(
                    "Collision mask for {:?} cannot be built: TiledMapAsset {:?} failed to load ({states:?})",
                    key, handle,
                );
            }
        }
    }
}

/// Collect the small fixed set of map handles required before gameplay.
pub fn required_collision_handles(
    level_assets: &LevelAssets,
    interior_assets: &InteriorAssets,
) -> Vec<(CollisionMapKey, Handle<TiledMapAsset>)> {
    let mut out = Vec::with_capacity(REQUIRED_COLLISION_KEYS.len());
    out.push((CollisionMapKey::Overview, level_assets.map.clone()));
    for (id, handle) in interior_assets.collision_entries() {
        out.push((CollisionMapKey::Enclosure(id), handle));
    }
    out
}

fn build_collision_masks_on_gameplay_enter(
    mut masks: ResMut<CollisionMasks>,
    map_assets: Res<Assets<TiledMapAsset>>,
    level_assets: Res<LevelAssets>,
    interior_assets: Res<InteriorAssets>,
) {
    build_all_collision_masks(&mut masks, &map_assets, &level_assets, &interior_assets);
}

fn key_for_asset_id(
    asset_id: AssetId<TiledMapAsset>,
    level_assets: &LevelAssets,
    interior_assets: &InteriorAssets,
) -> Option<CollisionMapKey> {
    if asset_id == level_assets.map.id() {
        return Some(CollisionMapKey::Overview);
    }

    for (enclosure_id, map_handle) in interior_assets.collision_entries() {
        if asset_id == map_handle.id() {
            return Some(CollisionMapKey::Enclosure(enclosure_id));
        }
    }

    None
}

/// Whether a tileset tile carries a Tiled class property for `T` (`propertytype` in `.tsx`).
fn tile_has_tiled_component<T: TypePath>(tile: &tiled::Tile) -> bool {
    tile.properties.values().any(|property| {
        matches!(
            property,
            PropertyValue::ClassValue { property_type, .. } if property_type == T::type_path()
        )
    })
}

pub fn build_mask_for_asset(asset: &TiledMapAsset) -> HashSet<TilePosition> {
    let mut obstacles = HashSet::new();

    for layer in asset.map.layers() {
        let LayerType::Tiles(tile_layer) = layer.layer_type() else {
            continue;
        };

        asset.for_each_tile(&tile_layer, |layer_tile, _data, tile_pos, _idx| {
            if layer_tile
                .get_tile()
                .is_some_and(|t| tile_has_tiled_component::<Obstacle>(&t))
            {
                obstacles.insert(TilePosition {
                    x: tile_pos.x,
                    y: tile_pos.y,
                });
            }
        });
    }

    obstacles
}

fn rebuild_collision_masks_on_asset_events(
    mut masks: ResMut<CollisionMasks>,
    map_assets: Res<Assets<TiledMapAsset>>,
    level_assets: Option<Res<LevelAssets>>,
    interior_assets: Option<Res<InteriorAssets>>,
    mut asset_events: MessageReader<AssetEvent<TiledMapAsset>>,
) {
    let (Some(level_assets), Some(interior_assets)) = (level_assets, interior_assets) else {
        return;
    };

    for event in asset_events.read() {
        let (AssetEvent::Added { id } | AssetEvent::Modified { id }) = event else {
            continue;
        };

        let Some(asset) = map_assets.get(*id) else {
            continue;
        };

        let Some(key) = key_for_asset_id(*id, &level_assets, &interior_assets) else {
            continue;
        };

        masks
            .static_blocked
            .insert(key, build_mask_for_asset(asset));
    }
}

// ---------------------------------------------------------
// Dynamic obstacle ECS sync (in-room live entities -> save-backed tiles)
// ---------------------------------------------------------

#[derive(Component)]
struct TrackedDynamicObstacle {
    enclosure_id: EnclosureId,
    last_tile: TilePosition,
}

fn register_dynamic_obstacle_spawn(
    add: On<Add, PersistedDynamicObstacle>,
    mut commands: Commands,
    query: Query<
        (&DynamicObstacle, &CurrentTilePosition, &InEnclosure),
        (Added<PersistedDynamicObstacle>, With<DynamicObstacle>),
    >,
    mut enclosure_query: Query<(&EnclosureId, &mut DynamicObstacleTiles)>,
    static_masks: Res<CollisionMasks>,
) {
    let entity = add.entity;
    let Ok((_, tile_pos, in_enclosure)) = query.get(entity) else {
        return;
    };

    let enc_id = in_enclosure.0;
    let key = CollisionMapKey::Enclosure(enc_id);
    if static_masks.is_statically_walkable(key, tile_pos.0)
        && let Some((_, mut tiles)) = enclosure_query.iter_mut().find(|(id, _)| **id == enc_id)
    {
        tiles.insert(tile_pos.0);
    }

    commands.entity(entity).insert(TrackedDynamicObstacle {
        enclosure_id: enc_id,
        last_tile: tile_pos.0,
    });
}

fn sync_dynamic_obstacle_positions(
    mut moved: Query<
        (
            &DynamicObstacle,
            &CurrentTilePosition,
            &InEnclosure,
            &mut TrackedDynamicObstacle,
        ),
        (
            Changed<CurrentTilePosition>,
            With<PersistedDynamicObstacle>,
            With<DynamicObstacle>,
        ),
    >,
    mut enclosure_query: Query<(&EnclosureId, &mut DynamicObstacleTiles)>,
    static_masks: Res<CollisionMasks>,
) {
    for (_, tile_pos, in_enclosure, mut tracked) in &mut moved {
        if tracked.last_tile == tile_pos.0 {
            continue;
        }

        let enc_id = in_enclosure.0;
        let key = CollisionMapKey::Enclosure(enc_id);
        if let Some((_, mut tiles)) = enclosure_query.iter_mut().find(|(id, _)| **id == enc_id) {
            tiles.remove(tracked.last_tile);
            if static_masks.is_statically_walkable(key, tile_pos.0) {
                tiles.insert(tile_pos.0);
            }
        }

        tracked.last_tile = tile_pos.0;
    }
}

fn cleanup_dynamic_obstacle_tiles(
    mut removed: RemovedComponents<PersistedDynamicObstacle>,
    tracked: Query<&TrackedDynamicObstacle>,
    mut enclosure_query: Query<(&EnclosureId, &mut DynamicObstacleTiles)>,
) {
    for entity in removed.read() {
        let Ok(tracked) = tracked.get(entity) else {
            continue;
        };

        if let Some((_, mut tiles)) = enclosure_query
            .iter_mut()
            .find(|(id, _)| **id == tracked.enclosure_id)
        {
            tiles.remove(tracked.last_tile);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alveus_content::{POLLY_PLACEMENT, PUSH_POP_PLACEMENT};

    #[test]
    fn push_pop_wander_never_includes_feeding_dish() {
        let key = CollisionMapKey::Enclosure(EnclosureId::PushPopEnclosure);
        let mut masks = CollisionMasks::default();
        masks
            .static_blocked
            .insert(key, HashSet::from([TilePosition { x: 8, y: 6 }]));

        let dish = TilePosition { x: 8, y: 6 };
        assert!(
            !masks.is_statically_walkable(key, dish),
            "feeding dish tile must be blocked"
        );

        let from = TilePosition { x: 8, y: 5 };
        let bounds = PUSH_POP_PLACEMENT.wander_bounds;
        let neighbors = adjacent_tiles(from)
            .into_iter()
            .filter(|tile| {
                tile_in_bounds(*tile, bounds)
                    && is_walkable_with_dynamic(
                        &masks,
                        &[] as &[TilePosition],
                        &[] as &[TilePosition],
                        key,
                        *tile,
                    )
            })
            .collect::<Vec<_>>();

        assert!(
            !neighbors.contains(&dish),
            "wander candidates from (8,5) must not include the dish at (8,6)"
        );
    }

    /// Static obstacles for Polly's Nutrition House stations as laid out in
    /// `tools/gen_interiors.py` (open floor; no playpen fence).
    fn nutrition_playpen_static_blocked() -> HashSet<TilePosition> {
        HashSet::from([
            // Nesting station
            TilePosition { x: 9, y: 2 },
            // Enrichment post
            TilePosition { x: 7, y: 5 },
            // Feed bowl
            TilePosition { x: 8, y: 3 },
        ])
    }

    fn walkable_adjacent(
        masks: &CollisionMasks,
        key: CollisionMapKey,
        from: TilePosition,
        bounds: TileBounds,
    ) -> Vec<TilePosition> {
        adjacent_tiles(from)
            .into_iter()
            .filter(|tile| {
                tile_in_bounds(*tile, bounds)
                    && is_walkable_with_dynamic(
                        masks,
                        &[] as &[TilePosition],
                        &[] as &[TilePosition],
                        key,
                        *tile,
                    )
            })
            .collect()
    }

    #[test]
    fn polly_playpen_stations_reachable_and_home_has_wander_neighbors() {
        let key = CollisionMapKey::Enclosure(EnclosureId::NutritionHousePlaypen);
        let mut masks = CollisionMasks::default();
        masks
            .static_blocked
            .insert(key, nutrition_playpen_static_blocked());

        let bounds = POLLY_PLACEMENT.wander_bounds;
        let home = POLLY_PLACEMENT.home_position;
        let nesting = TilePosition { x: 9, y: 2 };
        let enrichment = TilePosition { x: 7, y: 5 };
        let bowl = TilePosition { x: 8, y: 3 };

        assert!(
            !masks.is_statically_walkable(key, nesting),
            "nesting box must be blocked"
        );
        assert!(
            !masks.is_statically_walkable(key, enrichment),
            "enrichment post must be blocked"
        );
        assert!(
            !masks.is_statically_walkable(key, bowl),
            "feed bowl must be blocked"
        );
        assert!(
            masks.is_statically_walkable(key, TilePosition { x: 6, y: 3 }),
            "former gate tile must be open floor"
        );
        assert!(
            masks.is_statically_walkable(key, home),
            "Polly home must be walkable"
        );

        let home_neighbors = walkable_adjacent(&masks, key, home, bounds);
        assert!(
            home_neighbors.len() >= 2,
            "Polly home needs ≥2 wander neighbors, got {home_neighbors:?}"
        );

        let nesting_adj = walkable_adjacent(&masks, key, nesting, bounds);
        assert!(
            !nesting_adj.is_empty(),
            "nesting box needs a walkable adjacent tile, got none"
        );

        let enrich_adj = walkable_adjacent(&masks, key, enrichment, bounds);
        assert!(
            !enrich_adj.is_empty(),
            "enrichment post needs a walkable adjacent tile, got none"
        );

        let bowl_adj = walkable_adjacent(&masks, key, bowl, bounds);
        assert!(
            !bowl_adj.is_empty(),
            "feed bowl needs a walkable adjacent tile, got none"
        );
    }

    #[test]
    fn live_obstacle_tile_blocks_movement() {
        let key = CollisionMapKey::Enclosure(EnclosureId::PushPopEnclosure);
        let mut masks = CollisionMasks::default();
        masks.static_blocked.insert(key, HashSet::new());

        let blocked = TilePosition { x: 7, y: 5 };
        assert!(is_walkable_with_dynamic(
            &masks,
            &[] as &[TilePosition],
            &[] as &[TilePosition],
            key,
            blocked,
        ));
        assert!(!is_walkable_with_dynamic(
            &masks,
            &[] as &[TilePosition],
            [blocked],
            key,
            blocked,
        ));
    }

    fn sample_failure(key: CollisionMapKey) -> CollisionLoadFailure {
        CollisionLoadFailure {
            key,
            asset_path: key.asset_path().to_string(),
            reason: COLLISION_LOAD_REASON_RECURSIVE_DEPENDENCY_FAILED.to_string(),
        }
    }

    #[test]
    fn collision_load_failures_dedupe_by_key() {
        let mut failures = CollisionLoadFailures::default();
        let key = CollisionMapKey::Overview;

        assert!(failures.record(sample_failure(key)));
        assert!(!failures.record(sample_failure(key)));
        assert_eq!(failures.entries.len(), 1);
        assert!(failures.contains_key(key));
    }

    #[test]
    fn collision_load_failures_clear_removes_stale_entries() {
        let mut failures = CollisionLoadFailures::default();
        assert!(failures.record(sample_failure(CollisionMapKey::Overview)));
        assert!(!failures.is_empty());

        failures.clear();
        assert!(failures.is_empty());
        assert!(failures.record(sample_failure(CollisionMapKey::Overview)));
        assert_eq!(failures.entries.len(), 1);
    }

    #[test]
    fn collision_load_failures_toast_message_is_concise() {
        let mut failures = CollisionLoadFailures::default();
        for key in [
            CollisionMapKey::Overview,
            CollisionMapKey::Enclosure(EnclosureId::NutritionHousePlaypen),
            CollisionMapKey::Enclosure(EnclosureId::PushPopEnclosure),
        ] {
            assert!(failures.record(sample_failure(key)));
        }

        let toast = failures.toast_message();
        assert!(toast.contains("Could not load 3 maps"));
        assert!(!toast.contains('\n'));
        assert!(toast.chars().count() < 80);

        let detail = failures.loading_detail_message();
        assert!(detail.contains("Could not load a required map"));
        assert!(detail.chars().count() <= MAX_UI_MESSAGE_CHARS);
    }

    #[test]
    fn collision_load_failures_player_message_is_bounded() {
        let mut failures = CollisionLoadFailures::default();
        for key in [
            CollisionMapKey::Overview,
            CollisionMapKey::Enclosure(EnclosureId::NutritionHousePlaypen),
            CollisionMapKey::Enclosure(EnclosureId::PushPopEnclosure),
            CollisionMapKey::Enclosure(EnclosureId::Pasture),
            CollisionMapKey::Enclosure(EnclosureId::ReptileEnclosure),
        ] {
            assert!(failures.record(sample_failure(key)));
        }

        let msg = failures.loading_detail_message();
        assert!(msg.contains("Could not load a required map"));
        assert!(msg.chars().count() <= MAX_UI_MESSAGE_CHARS);
    }
}
