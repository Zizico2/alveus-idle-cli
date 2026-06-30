//! Collision: static masks from Tiled assets + save-backed dynamic blocked tiles.

use std::collections::{HashMap, HashSet};
use std::borrow::Borrow;

use bevy::prelude::*;
use bevy::reflect::TypePath;
use bevy_ecs_tiled::prelude::*;
use moonshine_save::prelude::{Save, Unload};
use rand::prelude::*;
use tiled::{LayerType, PropertyValue};

use crate::components::{
    CurrentTilePosition, DynamicObstacle, InEnclosure, Obstacle, PersistedDynamicObstacle,
    TilePosition,
};
use crate::content::{adjacent_tiles, animal_default_placement, enclosure_for_animal, tile_in_bounds, TileBounds};
use crate::demo::level::{InteriorAssets, LevelAssets};
use crate::screens::{InRoom, Screen};
use crate::stats::EnclosureId;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
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
}

pub fn enclosure_for_room(room: InRoom) -> EnclosureId {
    match room {
        InRoom::NutritionHouse => EnclosureId::NutritionHousePlaypen,
        InRoom::PushPopEnclosure => EnclosureId::PushPopEnclosure,
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

pub fn collision_key_for_animal(animal_id: crate::stats::AnimalId) -> Option<CollisionMapKey> {
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
    let handles = std::iter::once((
        CollisionMapKey::Overview,
        level_assets.map.clone(),
    ))
    .chain(
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
        && let Some((_, mut tiles)) = enclosure_query
            .iter_mut()
            .find(|(id, _)| **id == enc_id)
        {
            tiles.insert(tile_pos.0);
        }

    commands
        .entity(entity)
        .insert(TrackedDynamicObstacle {
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
        if let Some((_, mut tiles)) = enclosure_query
            .iter_mut()
            .find(|(id, _)| **id == enc_id)
        {
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
    use crate::components::TilePosition;

    #[test]
    fn push_pop_wander_never_includes_feeding_dish() {
        let key = CollisionMapKey::Enclosure(EnclosureId::PushPopEnclosure);
        let mut masks = CollisionMasks::default();
        masks.static_blocked.insert(
            key,
            HashSet::from([TilePosition { x: 8, y: 6 }]),
        );

        let dish = TilePosition { x: 8, y: 6 };
        assert!(
            !masks.is_statically_walkable(key, dish),
            "feeding dish tile must be blocked"
        );

        let from = TilePosition { x: 8, y: 5 };
        let bounds = crate::content::PUSH_POP_PLACEMENT.wander_bounds;
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
            &[blocked],
            key,
            blocked,
        ));
    }
}
