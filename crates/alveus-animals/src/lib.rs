use alveus_app::{InRoom, Screen};
use alveus_collision::{
    CollisionMapKey, CollisionMasks, DynamicObstacleTiles, LiveObstacleItem, collision_key_for_animal,
    is_walkable, walkable_neighbors,
};
use alveus_components::{
    CurrentTilePosition, DesiredTilePosition, DynamicObstacle, InEnclosure, TILE_SIZE, TilePosition,
};
use alveus_content::{TileBounds, animal_default_placement};
use alveus_stats::{AnimalBackgroundWander, AnimalEnclosure, AnimalTilePosition};
use alveus_types::{AnimalId, EnclosureId};
use bevy::prelude::*;
use rand::prelude::*;

pub struct AnimalsPlugin;

impl Plugin for AnimalsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                tick_animal_wander,
                start_animal_movement,
                apply_animal_movement,
                sync_npc_position_to_stats,
            )
                .run_if(in_state(Screen::InRoom(InRoom::PushPopEnclosure))),
        )
        .add_systems(
            Update,
            tick_background_animal_wander.run_if(in_state(Screen::Gameplay)),
        );
    }
}

#[derive(Component, Debug)]
pub struct AnimalNpc {
    pub animal_id: AnimalId,
}

#[derive(Component, Debug)]
pub struct WanderInZone {
    pub bounds: TileBounds,
    pub idle_timer: Timer,
    pub move_timer: Timer,
    pub target: Option<TilePosition>,
}

impl WanderInZone {
    pub fn new(bounds: TileBounds) -> Self {
        Self {
            bounds,
            idle_timer: Timer::from_seconds(2.0, TimerMode::Repeating),
            move_timer: Timer::from_seconds(0.35, TimerMode::Once),
            target: None,
        }
    }
}

pub fn spawn_push_pop_npc(
    parent: &mut ChildSpawnerCommands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<ColorMaterial>,
    tile: TilePosition,
) {
    let placement =
        animal_default_placement(AnimalId::PushPop).expect("Push Pop must have placement config");
    let mesh = meshes.add(Circle::new(14.0));
    let material = materials.add(Color::srgb(0.45, 0.55, 0.30));

    parent.spawn((
        Name::new("Push Pop"),
        AnimalNpc {
            animal_id: AnimalId::PushPop,
        },
        DynamicObstacle,
        InEnclosure(EnclosureId::PushPopEnclosure),
        WanderInZone::new(placement.wander_bounds),
        CurrentTilePosition(tile),
        DesiredTilePosition(tile),
        Mesh2d(mesh),
        MeshMaterial2d(material),
        Transform::from_xyz(
            tile.x as f32 * TILE_SIZE as f32,
            tile.y as f32 * TILE_SIZE as f32,
            0.5,
        ),
    ));
}

fn tick_background_animal_wander(
    time: Res<Time>,
    masks: Res<CollisionMasks>,
    persisted_obstacles: Query<(&EnclosureId, &DynamicObstacleTiles)>,
    live_obstacles: Query<LiveObstacleItem<'_>>,
    mut query: Query<(
        Entity,
        &AnimalEnclosure,
        &mut AnimalTilePosition,
        &mut AnimalBackgroundWander,
    )>,
) {
    for (entity, enclosure, mut pos, mut wander) in &mut query {
        let key = CollisionMapKey::Enclosure(enclosure.0);
        if !masks.contains(key) {
            continue;
        }

        wander.idle_timer.tick(time.delta());
        if !wander.idle_timer.just_finished() {
            continue;
        }

        let mut rng = rand::rng();
        let candidates = walkable_neighbors(
            pos.0,
            wander.bounds,
            key,
            &masks,
            &persisted_obstacles,
            &live_obstacles,
            Some(entity),
        );

        if let Some(target) = candidates.choose(&mut rng).copied() {
            pos.0 = target;
        }
    }
}

fn sync_npc_position_to_stats(
    npc_query: Query<(&AnimalNpc, &CurrentTilePosition), Changed<CurrentTilePosition>>,
    mut stats_query: Query<(&AnimalId, &mut AnimalTilePosition)>,
) {
    for (npc, pos) in &npc_query {
        for (id, mut saved) in &mut stats_query {
            if *id == npc.animal_id {
                saved.0 = pos.0;
            }
        }
    }
}

fn tick_animal_wander(
    time: Res<Time>,
    masks: Res<CollisionMasks>,
    persisted_obstacles: Query<(&EnclosureId, &DynamicObstacleTiles)>,
    live_obstacles: Query<LiveObstacleItem<'_>>,
    mut query: Query<(Entity, &AnimalNpc, &CurrentTilePosition, &mut WanderInZone)>,
) {
    for (entity, npc, pos, mut wander) in &mut query {
        let Some(key) = collision_key_for_animal(npc.animal_id) else {
            continue;
        };
        if !masks.contains(key) {
            continue;
        }

        if wander.target.is_some() {
            continue;
        }

        wander.idle_timer.tick(time.delta());
        if !wander.idle_timer.just_finished() {
            continue;
        }

        let mut rng = rand::rng();
        let candidates = walkable_neighbors(
            pos.0,
            wander.bounds,
            key,
            &masks,
            &persisted_obstacles,
            &live_obstacles,
            Some(entity),
        );

        if let Some(target) = candidates.choose(&mut rng).copied() {
            wander.target = Some(target);
            wander.move_timer.reset();
        }
    }
}

fn start_animal_movement(
    time: Res<Time>,
    masks: Res<CollisionMasks>,
    persisted_obstacles: Query<(&EnclosureId, &DynamicObstacleTiles)>,
    live_obstacles: Query<LiveObstacleItem<'_>>,
    mut query: Query<(
        Entity,
        &AnimalNpc,
        &CurrentTilePosition,
        &mut DesiredTilePosition,
        &mut WanderInZone,
        &mut Transform,
    )>,
) {
    for (entity, npc, current, mut desired, mut wander, mut transform) in &mut query {
        let Some(key) = collision_key_for_animal(npc.animal_id) else {
            continue;
        };

        let Some(target) = wander.target else {
            continue;
        };

        if current.0 == target {
            wander.target = None;
            continue;
        }

        wander.move_timer.tick(time.delta());
        if wander.move_timer.is_finished() {
            if masks.contains(key)
                && is_walkable(
                    &masks,
                    &persisted_obstacles,
                    &live_obstacles,
                    key,
                    target,
                    Some(entity),
                )
            {
                desired.0 = target;
            }
            wander.target = None;
            continue;
        }

        let progress = wander.move_timer.fraction();
        let start = tile_to_world(current.0);
        let end = tile_to_world(target);
        transform.translation.x = start.x + (end.x - start.x) * progress;
        transform.translation.y = start.y + (end.y - start.y) * progress;
    }
}

fn apply_animal_movement(
    mut query: Query<(
        &mut CurrentTilePosition,
        &DesiredTilePosition,
        &mut Transform,
        &mut WanderInZone,
    )>,
) {
    for (mut current, desired, mut transform, mut wander) in &mut query {
        if current.0 == desired.0 {
            continue;
        }

        current.0 = desired.0;
        let world = tile_to_world(current.0);
        transform.translation.x = world.x;
        transform.translation.y = world.y;
        wander.move_timer.reset();
    }
}

fn tile_to_world(tile: TilePosition) -> Vec2 {
    Vec2::new(
        tile.x as f32 * TILE_SIZE as f32,
        tile.y as f32 * TILE_SIZE as f32,
    )
}
