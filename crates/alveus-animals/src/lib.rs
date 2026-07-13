use alveus_app::{Screen, tile_interaction_enabled};
use alveus_collision::{
    CollisionMapKey, CollisionMasks, DynamicObstacleTiles, LiveObstacleItem,
    collision_key_for_animal, is_walkable, walkable_neighbors,
};
use alveus_components::{
    CurrentTilePosition, DesiredTilePosition, DynamicObstacle, InEnclosure, TILE_SIZE, TilePosition,
};
use alveus_content::{TileBounds, animal_default_placement};
use alveus_stats::{AnimalBackgroundWander, AnimalEnclosure, AnimalTilePosition};
use alveus_types::{AnimalId, EnclosureId};
use bevy::prelude::*;
use rand::prelude::*;

/// Ordered stages for foreground NPC locomotion. Driven by [`AnimalNpc`]
/// presence — not by which `InRoom` variant is active — so future resident rooms
/// do not need animal-system schedule edits.
#[derive(SystemSet, Debug, Clone, Copy, Eq, PartialEq, Hash)]
enum AnimalNpcSystems {
    ChooseTarget,
    StartMove,
    ApplyMove,
    PersistPosition,
}

pub struct AnimalsPlugin;

impl Plugin for AnimalsPlugin {
    fn build(&self, app: &mut App) {
        app.configure_sets(
            Update,
            (
                AnimalNpcSystems::ChooseTarget,
                AnimalNpcSystems::StartMove,
                AnimalNpcSystems::ApplyMove,
                AnimalNpcSystems::PersistPosition,
            )
                .chain()
                .run_if(any_with_component::<AnimalNpc>.and_then(tile_interaction_enabled)),
        )
        .add_systems(
            Update,
            (
                tick_animal_wander.in_set(AnimalNpcSystems::ChooseTarget),
                start_animal_movement.in_set(AnimalNpcSystems::StartMove),
                apply_animal_movement.in_set(AnimalNpcSystems::ApplyMove),
                sync_npc_position_to_stats.in_set(AnimalNpcSystems::PersistPosition),
            ),
        )
        .add_systems(
            Update,
            tick_background_animal_wander
                .run_if(in_state(Screen::Gameplay).and_then(tile_interaction_enabled)),
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
    spawn_animal_npc(
        parent,
        meshes,
        materials,
        tile,
        AnimalId::PushPop,
        EnclosureId::PushPopEnclosure,
        "Push Pop",
        Color::srgb(0.45, 0.55, 0.30),
    );
}

pub fn spawn_polly_npc(
    parent: &mut ChildSpawnerCommands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<ColorMaterial>,
    tile: TilePosition,
) {
    spawn_animal_npc(
        parent,
        meshes,
        materials,
        tile,
        AnimalId::Polly,
        EnclosureId::NutritionHousePlaypen,
        "Polly",
        Color::srgb(0.92, 0.88, 0.78),
    );
}

fn spawn_animal_npc(
    parent: &mut ChildSpawnerCommands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<ColorMaterial>,
    tile: TilePosition,
    animal_id: AnimalId,
    enclosure_id: EnclosureId,
    name: &str,
    color: Color,
) {
    let placement = animal_default_placement(animal_id).expect("animal must have placement config");
    let mesh = meshes.add(Circle::new(14.0));
    let material = materials.add(color);

    parent.spawn((
        Name::new(name.to_string()),
        AnimalNpc { animal_id },
        DynamicObstacle,
        InEnclosure(enclosure_id),
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

#[cfg(test)]
mod tests {
    use super::*;
    use alveus_app::{InRoom, Menu, Screen};
    use alveus_content::{POLLY_PLACEMENT, PUSH_POP_PLACEMENT};
    use bevy::state::app::StatesPlugin;
    use bevy::time::TimeUpdateStrategy;
    use std::collections::HashSet;
    use std::time::Duration;

    const STEP: Duration = Duration::from_millis(50);

    fn open_floor_mask(key: CollisionMapKey) -> CollisionMasks {
        let mut masks = CollisionMasks::default();
        // Empty blocked set ⇒ every tile is statically walkable for this key.
        masks.set_static_mask(key, HashSet::new());
        masks
    }

    fn animal_app(screen: Screen, key: CollisionMapKey) -> App {
        let mut app = App::new();
        app.add_plugins(StatesPlugin);
        app.add_plugins(alveus_app::plugin);
        app.add_plugins(MinimalPlugins);
        app.insert_resource(TimeUpdateStrategy::ManualDuration(STEP));
        app.insert_resource(open_floor_mask(key));
        app.add_plugins(AnimalsPlugin);
        app.insert_resource(NextState::Pending(screen));
        app.update();
        app
    }

    fn spawn_stats(app: &mut App, animal_id: AnimalId, tile: TilePosition, bounds: TileBounds) {
        let mut wander = AnimalBackgroundWander::new(bounds);
        wander.idle_timer = Timer::from_seconds(0.05, TimerMode::Repeating);
        app.world_mut().spawn((
            animal_id,
            AnimalEnclosure(enclosure_for_test(animal_id)),
            AnimalTilePosition(tile),
            wander,
        ));
    }

    fn enclosure_for_test(animal_id: AnimalId) -> EnclosureId {
        match animal_id {
            AnimalId::Polly => EnclosureId::NutritionHousePlaypen,
            AnimalId::PushPop => EnclosureId::PushPopEnclosure,
            AnimalId::Stompy => EnclosureId::Pasture,
            AnimalId::Georgie | AnimalId::Siren => EnclosureId::ReptileEnclosure,
        }
    }

    fn spawn_npc(
        app: &mut App,
        animal_id: AnimalId,
        tile: TilePosition,
        bounds: TileBounds,
    ) -> Entity {
        let mut wander = WanderInZone::new(bounds);
        // Short timers so ManualDuration steps can finish idle/move quickly.
        wander.idle_timer = Timer::from_seconds(0.05, TimerMode::Repeating);
        wander.move_timer = Timer::from_seconds(0.05, TimerMode::Once);
        app.world_mut()
            .spawn((
                AnimalNpc { animal_id },
                DynamicObstacle,
                InEnclosure(enclosure_for_test(animal_id)),
                wander,
                CurrentTilePosition(tile),
                DesiredTilePosition(tile),
                Transform::from_xyz(
                    tile.x as f32 * TILE_SIZE as f32,
                    tile.y as f32 * TILE_SIZE as f32,
                    0.5,
                ),
            ))
            .id()
    }

    fn advance(app: &mut App, frames: usize) {
        for _ in 0..frames {
            app.update();
        }
    }

    #[test]
    fn care_picker_freezes_foreground_animal_movement() {
        let key = CollisionMapKey::Enclosure(EnclosureId::PushPopEnclosure);
        let mut app = animal_app(Screen::InRoom(InRoom::PushPopEnclosure), key);
        let placement = PUSH_POP_PLACEMENT;
        let animal = spawn_npc(
            &mut app,
            AnimalId::PushPop,
            placement.home_position,
            placement.wander_bounds,
        );

        app.world_mut()
            .resource_mut::<NextState<Menu>>()
            .set(Menu::CareItemPicker);
        app.update();
        let before = app.world().get::<CurrentTilePosition>(animal).unwrap().0;
        advance(&mut app, 5);

        assert_eq!(
            app.world().get::<CurrentTilePosition>(animal).unwrap().0,
            before
        );
        assert_eq!(
            app.world().get::<DesiredTilePosition>(animal).unwrap().0,
            before
        );
        assert!(
            app.world()
                .get::<WanderInZone>(animal)
                .unwrap()
                .target
                .is_none()
        );
    }

    #[test]
    fn care_picker_freezes_background_animal_movement() {
        let key = CollisionMapKey::Enclosure(EnclosureId::NutritionHousePlaypen);
        let mut app = animal_app(Screen::Gameplay, key);
        let placement = POLLY_PLACEMENT;
        spawn_stats(
            &mut app,
            AnimalId::Polly,
            placement.home_position,
            placement.wander_bounds,
        );

        app.world_mut()
            .resource_mut::<NextState<Menu>>()
            .set(Menu::CareItemPicker);
        app.update();
        let before = saved_tile(&app, AnimalId::Polly);
        advance(&mut app, 5);

        assert_eq!(saved_tile(&app, AnimalId::Polly), before);
    }

    fn saved_tile(app: &App, animal_id: AnimalId) -> TilePosition {
        app.world()
            .iter_entities()
            .filter_map(|entity| {
                let id = entity.get::<AnimalId>()?;
                let pos = entity.get::<AnimalTilePosition>()?;
                (*id == animal_id).then_some(pos.0)
            })
            .next()
            .expect("saved animal tile")
    }

    fn npc_tile(app: &App, entity: Entity) -> TilePosition {
        app.world()
            .get::<CurrentTilePosition>(entity)
            .expect("npc CurrentTilePosition")
            .0
    }

    #[test]
    fn no_npc_world_updates_safely() {
        let mut app = animal_app(
            Screen::InRoom(InRoom::NutritionHouse),
            CollisionMapKey::Enclosure(EnclosureId::NutritionHousePlaypen),
        );
        spawn_stats(
            &mut app,
            AnimalId::Polly,
            POLLY_PLACEMENT.home_position,
            POLLY_PLACEMENT.wander_bounds,
        );
        advance(&mut app, 10);
        assert_eq!(
            saved_tile(&app, AnimalId::Polly),
            POLLY_PLACEMENT.home_position,
            "background wander must not run off Gameplay; foreground must no-op without AnimalNpc"
        );
    }

    #[test]
    fn foreground_runs_for_every_in_room_variant_with_synthetic_npc() {
        // Collision keys come from the animal placement, not the room enum. The
        // point is that every InRoom variant still schedules foreground systems
        // when an AnimalNpc exists — including reserved Pasture/Reptile.
        let rooms = [
            (
                InRoom::NutritionHouse,
                AnimalId::Polly,
                POLLY_PLACEMENT.home_position,
                POLLY_PLACEMENT.wander_bounds,
            ),
            (
                InRoom::PushPopEnclosure,
                AnimalId::PushPop,
                PUSH_POP_PLACEMENT.home_position,
                PUSH_POP_PLACEMENT.wander_bounds,
            ),
            (
                InRoom::Pasture,
                AnimalId::Polly,
                POLLY_PLACEMENT.home_position,
                POLLY_PLACEMENT.wander_bounds,
            ),
            (
                InRoom::ReptileEnclosure,
                AnimalId::PushPop,
                PUSH_POP_PLACEMENT.home_position,
                PUSH_POP_PLACEMENT.wander_bounds,
            ),
        ];

        for (room, animal_id, home, bounds) in rooms {
            let key = collision_key_for_animal(animal_id).expect("test animals have placement");
            let mut app = animal_app(Screen::InRoom(room), key);
            spawn_stats(&mut app, animal_id, home, bounds);
            let npc = spawn_npc(&mut app, animal_id, home, bounds);

            let start = npc_tile(&app, npc);
            advance(&mut app, 40);
            let after = npc_tile(&app, npc);
            assert_ne!(
                after, start,
                "foreground wander should move NPC in {room:?}"
            );
            assert_eq!(
                saved_tile(&app, animal_id),
                after,
                "Changed tile must sync to stats in {room:?}"
            );
        }
    }

    #[test]
    fn background_and_foreground_do_not_both_mutate_same_animal_in_one_mode() {
        let key = CollisionMapKey::Enclosure(EnclosureId::NutritionHousePlaypen);
        let home = POLLY_PLACEMENT.home_position;
        let bounds = POLLY_PLACEMENT.wander_bounds;

        // Gameplay: background may move saved tile; no AnimalNpc ⇒ foreground idle.
        let mut gameplay = animal_app(Screen::Gameplay, key);
        spawn_stats(&mut gameplay, AnimalId::Polly, home, bounds);
        let before = saved_tile(&gameplay, AnimalId::Polly);
        advance(&mut gameplay, 80);
        let after_bg = saved_tile(&gameplay, AnimalId::Polly);
        assert_ne!(
            after_bg, before,
            "background wander should move on Gameplay"
        );
        assert!(
            gameplay
                .world_mut()
                .query::<&AnimalNpc>()
                .iter(gameplay.world())
                .next()
                .is_none()
        );

        // InRoom with NPC: foreground moves NPC + syncs; background must not run.
        let mut room = animal_app(Screen::InRoom(InRoom::NutritionHouse), key);
        spawn_stats(&mut room, AnimalId::Polly, home, bounds);
        let npc = spawn_npc(&mut room, AnimalId::Polly, home, bounds);
        let start_npc = npc_tile(&room, npc);
        let start_saved = saved_tile(&room, AnimalId::Polly);
        advance(&mut room, 40);
        let end_npc = npc_tile(&room, npc);
        let end_saved = saved_tile(&room, AnimalId::Polly);
        assert_ne!(end_npc, start_npc);
        assert_eq!(end_saved, end_npc);
        assert_ne!(
            end_saved, start_saved,
            "only foreground sync should have updated the saved tile while InRoom"
        );
    }

    #[test]
    fn two_npcs_iterate_independently_without_single_assumption() {
        let key = CollisionMapKey::Enclosure(EnclosureId::NutritionHousePlaypen);
        let mut app = animal_app(Screen::InRoom(InRoom::NutritionHouse), key);

        let polly_home = POLLY_PLACEMENT.home_position;
        let push_home = TilePosition { x: 7, y: 3 };
        spawn_stats(
            &mut app,
            AnimalId::Polly,
            polly_home,
            POLLY_PLACEMENT.wander_bounds,
        );
        spawn_stats(
            &mut app,
            AnimalId::PushPop,
            push_home,
            PUSH_POP_PLACEMENT.wander_bounds,
        );
        // Both NPCs share Polly's collision key (NutritionHousePlaypen) via AnimalId::Polly
        // placement lookup — use Polly id for movement eligibility, distinct saved ids via
        // separate stats. For true dual-id movement both need collision_key_for_animal.
        // Spawn one Polly-keyed and one PushPop-keyed NPC with Push Pop's enclosure mask too.
        app.world_mut()
            .resource_mut::<CollisionMasks>()
            .set_static_mask(
                CollisionMapKey::Enclosure(EnclosureId::PushPopEnclosure),
                HashSet::new(),
            );

        let polly_npc = spawn_npc(
            &mut app,
            AnimalId::Polly,
            polly_home,
            POLLY_PLACEMENT.wander_bounds,
        );
        let push_npc = spawn_npc(
            &mut app,
            AnimalId::PushPop,
            push_home,
            PUSH_POP_PLACEMENT.wander_bounds,
        );

        let polly_start = npc_tile(&app, polly_npc);
        let push_start = npc_tile(&app, push_npc);
        advance(&mut app, 50);

        let polly_after = npc_tile(&app, polly_npc);
        let push_after = npc_tile(&app, push_npc);
        assert_ne!(polly_after, polly_start);
        assert_ne!(push_after, push_start);
        assert_eq!(saved_tile(&app, AnimalId::Polly), polly_after);
        assert_eq!(saved_tile(&app, AnimalId::PushPop), push_after);

        let npc_count = app
            .world_mut()
            .query::<&AnimalNpc>()
            .iter(app.world())
            .count();
        assert_eq!(npc_count, 2);
    }

    #[test]
    fn npc_does_not_enter_static_or_dynamic_obstacles() {
        let key = CollisionMapKey::Enclosure(EnclosureId::NutritionHousePlaypen);
        let home = TilePosition { x: 8, y: 4 };
        let blocked = TilePosition { x: 8, y: 5 };
        let bounds = TileBounds {
            bottom_left: TilePosition { x: 8, y: 4 },
            top_right: TilePosition { x: 8, y: 5 },
        };

        let mut app = animal_app(Screen::InRoom(InRoom::NutritionHouse), key);
        app.world_mut()
            .resource_mut::<CollisionMasks>()
            .set_static_mask(key, HashSet::from([blocked]));

        spawn_stats(&mut app, AnimalId::Polly, home, bounds);
        let npc = spawn_npc(&mut app, AnimalId::Polly, home, bounds);
        // Seed a desired target into the blocked tile and force apply path via start_animal_movement.
        {
            let mut wander = app.world_mut().get_mut::<WanderInZone>(npc).unwrap();
            wander.target = Some(blocked);
            wander.move_timer = Timer::from_seconds(0.01, TimerMode::Once);
            // Mark finished on next tick.
            wander.move_timer.tick(Duration::from_millis(20));
        }

        advance(&mut app, 5);
        assert_eq!(
            npc_tile(&app, npc),
            home,
            "NPC must remain on walkable tile when target is statically blocked"
        );

        // Clear static block but add a live dynamic obstacle on the target tile.
        app.world_mut()
            .resource_mut::<CollisionMasks>()
            .set_static_mask(key, HashSet::new());
        app.world_mut().spawn((
            DynamicObstacle,
            CurrentTilePosition(blocked),
            InEnclosure(EnclosureId::NutritionHousePlaypen),
        ));
        {
            let mut wander = app.world_mut().get_mut::<WanderInZone>(npc).unwrap();
            wander.target = Some(blocked);
            wander.move_timer = Timer::from_seconds(0.01, TimerMode::Once);
            wander.move_timer.tick(Duration::from_millis(20));
        }
        advance(&mut app, 5);
        assert_eq!(
            npc_tile(&app, npc),
            home,
            "NPC must not step onto a live dynamic obstacle"
        );
    }

    #[test]
    fn gameplay_to_room_to_gameplay_persists_final_tile() {
        let key = CollisionMapKey::Enclosure(EnclosureId::NutritionHousePlaypen);
        let home = POLLY_PLACEMENT.home_position;
        let bounds = POLLY_PLACEMENT.wander_bounds;

        let mut app = animal_app(Screen::Gameplay, key);
        spawn_stats(&mut app, AnimalId::Polly, home, bounds);

        // Background moves while on overview.
        advance(&mut app, 80);
        let after_background = saved_tile(&app, AnimalId::Polly);
        assert_ne!(after_background, home);

        // Enter room: stop background, spawn NPC at saved tile, foreground may move further.
        app.insert_resource(NextState::Pending(Screen::InRoom(InRoom::NutritionHouse)));
        app.update();
        let npc = spawn_npc(&mut app, AnimalId::Polly, after_background, bounds);
        assert_eq!(npc_tile(&app, npc), after_background);

        advance(&mut app, 40);
        let in_room_tile = npc_tile(&app, npc);
        assert_eq!(
            saved_tile(&app, AnimalId::Polly),
            in_room_tile,
            "foreground sync must persist the NPC tile before exit"
        );

        // Despawn while still InRoom (mirrors DespawnOnExit during transition) so the
        // saved tile is frozen until Gameplay background wander resumes.
        app.world_mut().entity_mut(npc).despawn();
        assert_eq!(saved_tile(&app, AnimalId::Polly), in_room_tile);
        assert!(
            app.world()
                .iter_entities()
                .find(|e| e.get::<AnimalNpc>().is_some())
                .is_none(),
            "no stale NPC after leaving the room"
        );

        app.insert_resource(NextState::Pending(Screen::Gameplay));
        app.update();
        assert_eq!(
            *app.world().resource::<State<Screen>>().get(),
            Screen::Gameplay
        );

        // Background resumes from the persisted tile (may already step on the enter frame).
        let after_exit = saved_tile(&app, AnimalId::Polly);
        advance(&mut app, 20);
        let resumed = saved_tile(&app, AnimalId::Polly);
        assert!(
            resumed != in_room_tile || after_exit != in_room_tile,
            "background wander should resume after returning to Gameplay \
             (in_room={in_room_tile:?}, after_exit={after_exit:?}, resumed={resumed:?})"
        );
    }

    #[test]
    fn leaving_room_for_non_gameplay_screen_leaves_no_npc() {
        let key = CollisionMapKey::Enclosure(EnclosureId::NutritionHousePlaypen);
        let mut app = animal_app(Screen::InRoom(InRoom::NutritionHouse), key);
        spawn_stats(
            &mut app,
            AnimalId::Polly,
            POLLY_PLACEMENT.home_position,
            POLLY_PLACEMENT.wander_bounds,
        );
        let npc = spawn_npc(
            &mut app,
            AnimalId::Polly,
            POLLY_PLACEMENT.home_position,
            POLLY_PLACEMENT.wander_bounds,
        );
        advance(&mut app, 20);
        let last = npc_tile(&app, npc);
        assert_eq!(saved_tile(&app, AnimalId::Polly), last);

        app.world_mut().entity_mut(npc).despawn();
        app.insert_resource(NextState::Pending(Screen::Title));
        app.update();
        advance(&mut app, 10);

        assert!(
            app.world_mut()
                .query::<&AnimalNpc>()
                .iter(app.world())
                .next()
                .is_none()
        );
        assert!(
            app.world_mut()
                .query::<&DynamicObstacle>()
                .iter(app.world())
                .next()
                .is_none(),
            "no stale dynamic obstacle from the despawned NPC"
        );
        assert_eq!(saved_tile(&app, AnimalId::Polly), last);
    }

    #[test]
    fn spawn_helpers_attach_required_npc_components() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.init_resource::<Assets<Mesh>>();
        app.init_resource::<Assets<ColorMaterial>>();

        let tile = TilePosition { x: 8, y: 4 };
        let parent = app.world_mut().spawn_empty().id();

        {
            let world = app.world_mut();
            world.resource_scope(|world, mut meshes: Mut<Assets<Mesh>>| {
                world.resource_scope(|world, mut materials: Mut<Assets<ColorMaterial>>| {
                    let mut commands = world.commands();
                    let mut parent_cmds = commands.entity(parent);
                    parent_cmds.with_children(|child| {
                        spawn_polly_npc(child, &mut meshes, &mut materials, tile);
                        spawn_push_pop_npc(child, &mut meshes, &mut materials, tile);
                    });
                });
            });
            world.flush();
        }

        let npcs: Vec<(AnimalId, EnclosureId)> = app
            .world_mut()
            .query::<(&AnimalNpc, &InEnclosure)>()
            .iter(app.world())
            .map(|(npc, enc)| (npc.animal_id, enc.0))
            .collect();
        assert!(npcs.contains(&(AnimalId::Polly, EnclosureId::NutritionHousePlaypen)));
        assert!(npcs.contains(&(AnimalId::PushPop, EnclosureId::PushPopEnclosure)));

        for entity in app
            .world_mut()
            .query_filtered::<Entity, With<AnimalNpc>>()
            .iter(app.world())
            .collect::<Vec<_>>()
        {
            assert!(app.world().get::<DynamicObstacle>(entity).is_some());
            assert!(app.world().get::<WanderInZone>(entity).is_some());
            assert!(app.world().get::<CurrentTilePosition>(entity).is_some());
            assert!(app.world().get::<DesiredTilePosition>(entity).is_some());
        }
    }

    #[test]
    fn in_room_without_npc_does_not_run_foreground_behavior() {
        let key = CollisionMapKey::Enclosure(EnclosureId::Pasture);
        let mut app = animal_app(Screen::InRoom(InRoom::Pasture), key);
        spawn_stats(
            &mut app,
            AnimalId::Stompy,
            TilePosition { x: 1, y: 1 },
            TileBounds {
                bottom_left: TilePosition { x: 0, y: 0 },
                top_right: TilePosition { x: 2, y: 2 },
            },
        );
        let before = saved_tile(&app, AnimalId::Stompy);
        advance(&mut app, 40);
        assert_eq!(
            saved_tile(&app, AnimalId::Stompy),
            before,
            "InRoom alone must not move saved positions without an AnimalNpc"
        );
    }
}
