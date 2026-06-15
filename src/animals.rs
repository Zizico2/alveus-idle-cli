use bevy::prelude::*;
use rand::prelude::*;
use crate::components::{CurrentTilePosition, DesiredTilePosition, TilePosition};
use crate::content::{tile_in_bounds, PUSH_POP_PLACEMENT};
use crate::demo::level::TILE_SIZE;
use crate::interaction::AnimalFedEvent;
use crate::screens::{InRoom, Screen};
use crate::stats::AnimalId;

pub struct AnimalsPlugin;

impl Plugin for AnimalsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                tick_animal_wander,
                start_animal_movement,
                apply_animal_movement,
            )
                .run_if(in_state(Screen::InRoom(InRoom::PushPopEnclosure))),
        )
        .add_observer(react_to_feeding);
    }
}

#[derive(Component, Debug)]
pub struct AnimalNpc {
    pub animal_id: AnimalId,
}

#[derive(Component, Debug)]
pub struct WanderInZone {
    pub bounds: crate::content::TileBounds,
    pub idle_timer: Timer,
    pub move_timer: Timer,
    pub target: Option<TilePosition>,
    pub eating_timer: Option<Timer>,
}

impl WanderInZone {
    pub fn new(bounds: crate::content::TileBounds) -> Self {
        Self {
            bounds,
            idle_timer: Timer::from_seconds(2.0, TimerMode::Repeating),
            move_timer: Timer::from_seconds(0.35, TimerMode::Once),
            target: None,
            eating_timer: None,
        }
    }
}

pub fn spawn_push_pop_npc(
    parent: &mut ChildSpawnerCommands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<ColorMaterial>,
    home: TilePosition,
) {
    let placement = PUSH_POP_PLACEMENT;
    let mesh = meshes.add(Circle::new(14.0));
    let material = materials.add(Color::srgb(0.45, 0.55, 0.30));

    parent.spawn((
        Name::new("Push Pop"),
        AnimalNpc {
            animal_id: AnimalId::PushPop,
        },
        WanderInZone::new(placement.wander_bounds),
        CurrentTilePosition(home),
        DesiredTilePosition(home),
        Mesh2d(mesh),
        MeshMaterial2d(material),
        Transform::from_xyz(
            home.x as f32 * TILE_SIZE as f32,
            home.y as f32 * TILE_SIZE as f32,
            0.5,
        ),
    ));
}

fn tick_animal_wander(
    time: Res<Time>,
    mut query: Query<(&CurrentTilePosition, &mut WanderInZone)>,
) {
    for (pos, mut wander) in &mut query {
        if wander.eating_timer.is_some() || wander.target.is_some() {
            continue;
        }

        wander.idle_timer.tick(time.delta());
        if !wander.idle_timer.just_finished() {
            continue;
        }

        let mut rng = rand::rng();
        let candidates = adjacent_tiles(pos.0)
            .into_iter()
            .filter(|tile| tile_in_bounds(*tile, wander.bounds))
            .collect::<Vec<_>>();

        if let Some(target) = candidates.choose(&mut rng).copied() {
            wander.target = Some(target);
            wander.move_timer.reset();
        }
    }
}

fn start_animal_movement(
    time: Res<Time>,
    mut query: Query<(
        &CurrentTilePosition,
        &mut DesiredTilePosition,
        &mut WanderInZone,
        &mut Transform,
    )>,
) {
    for (current, mut desired, mut wander, mut transform) in &mut query {
        if let Some(timer) = wander.eating_timer.as_mut() {
            timer.tick(time.delta());
            if timer.is_finished() {
                wander.eating_timer = None;
            }
            continue;
        }

        let Some(target) = wander.target else {
            continue;
        };

        if current.0 == target {
            wander.target = None;
            continue;
        }

        wander.move_timer.tick(time.delta());
        if wander.move_timer.is_finished() {
            desired.0 = target;
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

fn react_to_feeding(
    trigger: On<AnimalFedEvent>,
    mut query: Query<(
        &AnimalNpc,
        &mut WanderInZone,
        &mut DesiredTilePosition,
        &CurrentTilePosition,
    )>,
) {
    let event = trigger.event();
    for (npc, mut wander, mut desired, current) in &mut query {
        if npc.animal_id != event.animal {
            continue;
        }

        wander.target = None;
        wander.eating_timer = Some(Timer::from_seconds(1.5, TimerMode::Once));
        desired.0 = adjacent_toward(current.0, event.dish_position);
    }
}

fn adjacent_toward(from: TilePosition, goal: TilePosition) -> TilePosition {
    if from == goal {
        return from;
    }

    let dx = goal.x as i32 - from.x as i32;
    let dy = goal.y as i32 - from.y as i32;

    if dx.abs() >= dy.abs() && dx != 0 {
        TilePosition {
            x: from.x.saturating_add_signed(dx.signum()),
            y: from.y,
        }
    } else if dy != 0 {
        TilePosition {
            x: from.x,
            y: from.y.saturating_add_signed(dy.signum()),
        }
    } else {
        from
    }
}

fn adjacent_tiles(tile: TilePosition) -> [TilePosition; 4] {
    [
        TilePosition {
            x: tile.x.saturating_sub(1),
            y: tile.y,
        },
        TilePosition {
            x: tile.x + 1,
            y: tile.y,
        },
        TilePosition {
            x: tile.x,
            y: tile.y.saturating_sub(1),
        },
        TilePosition {
            x: tile.x,
            y: tile.y + 1,
        },
    ]
}

fn tile_to_world(tile: TilePosition) -> Vec2 {
    Vec2::new(
        tile.x as f32 * TILE_SIZE as f32,
        tile.y as f32 * TILE_SIZE as f32,
    )
}
