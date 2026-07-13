//! Poop spawning, wheelbarrow pickup, and compost-dump cleaning loop.

use std::collections::HashSet;

use bevy::prelude::*;
use itertools::iproduct;
use rand::prelude::*;
use rand::seq::IteratorRandom;

use alveus_app::{AppSystems, InRoom, Screen};
use alveus_collision::{
    CollisionMapKey, CollisionMasks, DynamicObstacleTiles, LiveObstacleItem, collision_ready,
    enclosure_for_room,
};
use alveus_components::{
    CurrentTilePosition, DynamicObstacle, InEnclosure, Interactable, LastPickupMessage, Player,
    TILE_SIZE, TilePosition,
};
use alveus_stats::{
    EnclosureStat, EnclosureStats, ImproveStatEvent, StatTarget, apply_offline_decay_system,
    tick_decay_system,
};
use alveus_types::{EnclosureId, Stat};

// Poop config + cleaning math live in `alveus-configs`; re-export so callers and
// tests keep a single `alveus_cleaning::*` entry point.
pub use alveus_components::PoopWheelbarrow;
pub use alveus_configs::{
    PoopConfig, WHEELBARROW_CAPACITY, cleanliness_after_threshold_decay,
    cleanliness_decay_with_poops, enclosure_cleanliness_decay_amount, poop_config_for,
    target_poop_count,
};

pub fn room_for_enclosure(id: EnclosureId) -> InRoom {
    match id {
        EnclosureId::PushPopEnclosure => InRoom::PushPopEnclosure,
        EnclosureId::NutritionHousePlaypen => InRoom::NutritionHouse,
        EnclosureId::Pasture => InRoom::Pasture,
        EnclosureId::ReptileEnclosure => InRoom::ReptileEnclosure,
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

fn live_obstacles_for_enclosure(
    enclosure_id: EnclosureId,
    live_obstacles: &Query<LiveObstacleItem<'_>>,
) -> HashSet<TilePosition> {
    live_obstacles
        .iter()
        .filter_map(|(_, _, pos, in_enc)| {
            in_enc
                .filter(|InEnclosure(id)| *id == enclosure_id)
                .map(|_| pos.0)
        })
        .collect()
}

fn pick_random_poop_tile_in_bounds(
    config: &PoopConfig,
    key: CollisionMapKey,
    masks: &CollisionMasks,
    blocked: &HashSet<TilePosition>,
    live: &HashSet<TilePosition>,
    exclude: Option<TilePosition>,
    rng: &mut impl Rng,
) -> Option<TilePosition> {
    let bl = config.spawn_bounds.bottom_left;
    let tr = config.spawn_bounds.top_right;

    iproduct!(bl.x..=tr.x, bl.y..=tr.y)
        .map(|(x, y)| TilePosition { x, y })
        .filter(|tile| exclude != Some(*tile))
        .filter(|tile| !blocked.contains(tile))
        .filter(|tile| masks.is_statically_walkable(key, *tile))
        .filter(|tile| !live.contains(tile))
        .choose(rng)
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
    let blocked: HashSet<TilePosition> = tiles.0.iter().copied().collect();
    let live = live_obstacles_for_enclosure(enclosure_id, live_obstacles);
    pick_random_poop_tile_in_bounds(config, key, masks, &blocked, &live, exclude, rng)
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
    cleanliness: Stat,
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
    let mut blocked: HashSet<TilePosition> = tiles.0.iter().copied().collect();
    let live = live_obstacles_for_enclosure(enclosure_id, live_obstacles);

    while (tiles.0.len() as u32) < target {
        let Some(tile) =
            pick_random_poop_tile_in_bounds(config, key, masks, &blocked, &live, exclude, rng)
        else {
            break;
        };
        tiles.insert(tile);
        blocked.insert(tile);
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
                    .after(tick_decay_system)
                    .after(apply_offline_decay_system),
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
    mut enclosure_query: Query<(&EnclosureId, &EnclosureStats, &mut DynamicObstacleTiles)>,
    mut commands: Commands,
    mut meshes: Option<ResMut<Assets<Mesh>>>,
    mut materials: Option<ResMut<Assets<ColorMaterial>>>,
) {
    let Some(masks) = masks else {
        return;
    };
    if !collision_ready(&masks) {
        return;
    }

    let player_tile = player_query.single().ok().map(|p| p.0);
    let mut rng = rand::rng();

    for (enclosure_id, stats, mut tiles) in enclosure_query.iter_mut() {
        let Some(config) = poop_config_for(*enclosure_id) else {
            continue;
        };

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
            && let (Some(meshes), Some(materials)) =
                (meshes.as_deref_mut(), materials.as_deref_mut())
        {
            for tile in spawned {
                spawn_poop_entity(&mut commands, meshes, materials, tile, *enclosure_id, room);
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

    if let Some(config) = poop_config_for(event.enclosure_id) {
        commands.trigger(ImproveStatEvent {
            target: StatTarget::Enclosure {
                id: event.enclosure_id,
                stat: EnclosureStat::Cleanliness,
            },
            amount: config.cleanliness_restore_per_poop.into(),
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

#[cfg(test)]
mod tests {
    use super::*;
    use alveus_types::TileBounds;
    use bevy::state::app::StatesPlugin;
    use rand::SeedableRng;
    use rand::rngs::StdRng;

    fn test_config() -> PoopConfig {
        PoopConfig {
            spawn_thresholds: &[Stat(800)],
            poop_decay_rate: 1.0,
            cleanliness_restore_per_poop: alveus_types::CleanStat(Stat(1)),
            spawn_bounds: TileBounds {
                bottom_left: TilePosition { x: 0, y: 0 },
                top_right: TilePosition { x: 2, y: 2 },
            },
        }
    }

    fn test_key() -> CollisionMapKey {
        CollisionMapKey::Enclosure(EnclosureId::PushPopEnclosure)
    }

    fn spawn_test_poop(
        mut commands: Commands,
        mut meshes: ResMut<Assets<Mesh>>,
        mut materials: ResMut<Assets<ColorMaterial>>,
    ) {
        spawn_poop_entity(
            &mut commands,
            &mut meshes,
            &mut materials,
            TilePosition::default(),
            EnclosureId::PushPopEnclosure,
            InRoom::PushPopEnclosure,
        );
    }

    #[test]
    fn runtime_poop_is_scoped_to_its_room() {
        let room = Screen::InRoom(InRoom::PushPopEnclosure);
        let mut app = App::new();
        app.add_plugins(StatesPlugin);
        app.add_plugins(MinimalPlugins);
        app.add_plugins(alveus_app::plugin);
        app.init_resource::<Assets<Mesh>>()
            .init_resource::<Assets<ColorMaterial>>()
            .add_systems(OnEnter(room), spawn_test_poop);

        app.world_mut()
            .resource_mut::<NextState<Screen>>()
            .set(room);
        app.update();
        assert_eq!(
            app.world_mut()
                .query_filtered::<Entity, With<PoopPile>>()
                .iter(app.world())
                .count(),
            1
        );

        app.world_mut()
            .resource_mut::<NextState<Screen>>()
            .set(Screen::Gameplay);
        app.update();
        assert!(
            app.world_mut()
                .query_filtered::<Entity, With<PoopPile>>()
                .iter(app.world())
                .next()
                .is_none()
        );
    }

    fn all_test_tiles() -> HashSet<TilePosition> {
        HashSet::from([
            TilePosition { x: 0, y: 0 },
            TilePosition { x: 1, y: 0 },
            TilePosition { x: 2, y: 0 },
            TilePosition { x: 0, y: 1 },
            TilePosition { x: 1, y: 1 },
            TilePosition { x: 2, y: 1 },
            TilePosition { x: 0, y: 2 },
            TilePosition { x: 1, y: 2 },
            TilePosition { x: 2, y: 2 },
        ])
    }

    #[test]
    fn pick_random_poop_tile_in_bounds_returns_none_when_fully_blocked_by_mask() {
        let config = test_config();
        let key = test_key();
        let mut masks = CollisionMasks::default();
        masks.set_static_mask(key, all_test_tiles());
        let blocked = HashSet::new();
        let live = HashSet::new();
        let mut rng = StdRng::seed_from_u64(1);

        assert!(
            pick_random_poop_tile_in_bounds(&config, key, &masks, &blocked, &live, None, &mut rng,)
                .is_none()
        );
    }

    #[test]
    fn pick_random_poop_tile_in_bounds_excludes_blocked_tiles() {
        let config = test_config();
        let key = test_key();
        let mut masks = CollisionMasks::default();
        masks.set_static_mask(key, HashSet::new());
        let blocked = HashSet::from([TilePosition { x: 0, y: 0 }, TilePosition { x: 2, y: 2 }]);
        let live = HashSet::new();
        let mut rng = StdRng::seed_from_u64(2);

        for _ in 0..20 {
            let tile = pick_random_poop_tile_in_bounds(
                &config, key, &masks, &blocked, &live, None, &mut rng,
            )
            .expect("open tiles remain");
            assert!(!blocked.contains(&tile));
        }
    }

    #[test]
    fn pick_random_poop_tile_in_bounds_excludes_live_tiles() {
        let config = test_config();
        let key = test_key();
        let mut masks = CollisionMasks::default();
        masks.set_static_mask(key, HashSet::new());
        let blocked = HashSet::new();
        let live = HashSet::from([TilePosition { x: 1, y: 1 }]);
        let mut rng = StdRng::seed_from_u64(3);

        for _ in 0..20 {
            let tile = pick_random_poop_tile_in_bounds(
                &config, key, &masks, &blocked, &live, None, &mut rng,
            )
            .expect("open tiles remain");
            assert!(!live.contains(&tile));
        }
    }

    #[test]
    fn pick_random_poop_tile_in_bounds_respects_exclude() {
        let config = test_config();
        let key = test_key();
        let mut masks = CollisionMasks::default();
        masks.set_static_mask(key, HashSet::new());
        let blocked = HashSet::new();
        let live = HashSet::new();
        let exclude = TilePosition { x: 1, y: 1 };
        let mut rng = StdRng::seed_from_u64(4);

        for _ in 0..20 {
            let tile = pick_random_poop_tile_in_bounds(
                &config,
                key,
                &masks,
                &blocked,
                &live,
                Some(exclude),
                &mut rng,
            )
            .expect("open tiles remain");
            assert_ne!(tile, exclude);
        }
    }

    #[test]
    fn pick_random_poop_tile_in_bounds_picks_valid_tile() {
        let config = test_config();
        let key = test_key();
        let mut masks = CollisionMasks::default();
        masks.set_static_mask(key, HashSet::new());
        let blocked = HashSet::new();
        let live = HashSet::new();
        let mut rng = StdRng::seed_from_u64(5);

        let tile =
            pick_random_poop_tile_in_bounds(&config, key, &masks, &blocked, &live, None, &mut rng)
                .expect("should pick from open grid");
        assert!(all_test_tiles().contains(&tile));
    }
}
