use alveus_animals::{spawn_polly_npc, spawn_push_pop_npc};
use alveus_app::{InRoom, Screen};
use alveus_collision::{
    CollisionMapKey, CollisionMasks, DynamicObstacleTiles, InteriorAssets, LiveObstacleItem,
    resolve_spawn_tile,
};
use alveus_components::{
    BuildingEntrance, CurrentTilePosition, DesiredTilePosition, Player, TILE_SIZE, TilePosition,
};
use alveus_configs::{
    NUTRITION_HOUSE_ROOM, OVERVIEW_PLAYER_SPAWN, PUSH_POP_ENCLOSURE_ROOM, animal_default_placement,
};
use alveus_content::default_tile_position;
use alveus_stats::{AnimalTilePosition, EnclosureId};
use alveus_types::AnimalId;
use bevy::prelude::*;
use bevy::state::state::FreelyMutableState;
use bevy_ecs_tiled::prelude::*;

use crate::player::player;
use crate::toast::despawn_active_toast;

#[derive(Resource, Debug, Clone, Copy, Reflect)]
#[reflect(Resource)]
pub struct PlayerSpawnPoint {
    pub position: TilePosition,
}

impl Default for PlayerSpawnPoint {
    fn default() -> Self {
        Self {
            position: OVERVIEW_PLAYER_SPAWN,
        }
    }
}

pub fn try_enter_room<S: States + FreelyMutableState>(
    player_entrance: &BuildingEntrance,
    required_entrance: BuildingEntrance,
    room_state: S,
    next_screen: &mut NextState<S>,
) -> bool {
    if *player_entrance == required_entrance {
        info!("Entering room interior state!");
        next_screen.set(room_state);
        true
    } else {
        false
    }
}

pub fn try_exit_room<S: States + FreelyMutableState>(
    player_pos: TilePosition,
    exit_door: TilePosition,
    exit_spawn: TilePosition,
    gameplay_state: S,
    spawn_point: &mut PlayerSpawnPoint,
    next_screen: &mut NextState<S>,
    force: bool,
) -> bool {
    let should_exit = force || (player_pos.x == exit_door.x && player_pos.y == exit_door.y);

    if should_exit {
        info!("Exiting room interior!");
        spawn_point.position = exit_spawn;
        next_screen.set(gameplay_state);
        true
    } else {
        false
    }
}

pub struct RoomConfig<S: States + FreelyMutableState> {
    pub room_state: S,
    pub gameplay_state: S,
    pub entrance: BuildingEntrance,
    pub enclosure_id: EnclosureId,
    pub room_spawn: TilePosition,
    pub exit_spawn: TilePosition,
    pub exit_door: TilePosition,
    pub get_interior_map: fn(&InteriorAssets) -> Handle<TiledMapAsset>,
    /// When set, spawn extras at a resolved tile for this resident animal.
    pub resident_animal: Option<AnimalId>,
    pub spawn_extras_fn:
        fn(&mut ChildSpawnerCommands, &mut Assets<Mesh>, &mut Assets<ColorMaterial>, TilePosition),
    pub room_title: String,
}

pub fn build_room<S: States + FreelyMutableState>(app: &mut App, config: RoomConfig<S>) {
    app.init_resource::<PlayerSpawnPoint>();

    let room_state = config.room_state.clone();
    let gameplay_state = config.gameplay_state.clone();
    let entrance = config.entrance;
    let enclosure_id = config.enclosure_id;
    let room_spawn = config.room_spawn;
    let exit_spawn = config.exit_spawn;
    let exit_door = config.exit_door;
    let get_interior_map = config.get_interior_map;
    let resident_animal = config.resident_animal;
    let spawn_extras_fn = config.spawn_extras_fn;
    let room_title = config.room_title;

    // OnEnter systems
    let enter_state = room_state.clone();

    let enter_state_for_spawner = enter_state.clone();
    let room_title_for_spawner = room_title.clone();

    let enter_state_for_ui = enter_state.clone();
    let room_title_for_ui = room_title.clone();

    app.add_systems(
        OnEnter(enter_state.clone()),
        (
            move |mut commands: Commands,
                  interior_assets: Res<InteriorAssets>,
                  masks: Res<CollisionMasks>,
                  persisted_obstacles: Query<(&EnclosureId, &DynamicObstacleTiles)>,
                  live_obstacles: Query<LiveObstacleItem<'_>>,
                  mut meshes: ResMut<Assets<Mesh>>,
                  mut materials: ResMut<Assets<ColorMaterial>>,
                  animal_positions: Query<(&AnimalId, &AnimalTilePosition)>| {
                let collision_key = CollisionMapKey::Enclosure(enclosure_id);

                commands
                    .spawn((
                        Name::new(format!("{} Room", room_title_for_spawner)),
                        Transform::default(),
                        Visibility::default(),
                        DespawnOnExit(enter_state_for_spawner.clone()),
                    ))
                    .with_children(|parent| {
                        parent.spawn((
                            player(&mut meshes, &mut materials, room_spawn),
                            CurrentTilePosition(room_spawn),
                            DesiredTilePosition(room_spawn),
                        ));

                        let map_handle = get_interior_map(&interior_assets);
                        spawn_interior_map(parent, map_handle);

                        if let Some(animal_id) = resident_animal {
                            let preferred = animal_positions
                                .iter()
                                .find(|(id, _)| **id == animal_id)
                                .map(|(_, pos)| pos.0)
                                .or_else(|| default_tile_position(animal_id))
                                .unwrap_or(room_spawn);

                            let wander_bounds = animal_default_placement(animal_id)
                                .map(|p| p.wander_bounds)
                                .unwrap_or_else(|| alveus_types::TileBounds {
                                    bottom_left: preferred,
                                    top_right: preferred,
                                });

                            let spawn_tile = resolve_spawn_tile(
                                preferred,
                                wander_bounds,
                                collision_key,
                                &masks,
                                &persisted_obstacles,
                                &live_obstacles,
                                None,
                            );
                            spawn_extras_fn(parent, &mut meshes, &mut materials, spawn_tile);
                        }
                    });
            },
            move |mut commands: Commands| {
                commands
                    .spawn((
                        Name::new("Room UI Root"),
                        Node {
                            position_type: PositionType::Absolute,
                            top: Val::Px(24.0),
                            left: Val::Px(24.0),
                            flex_direction: FlexDirection::Column,
                            row_gap: Val::Px(8.0),
                            ..default()
                        },
                        DespawnOnExit(enter_state_for_ui.clone()),
                    ))
                    .with_children(|parent| {
                        parent.spawn((
                            Text::new(room_title_for_ui.clone()),
                            TextFont::from_font_size(32.0),
                            TextColor(Color::srgb(0.2, 0.8, 0.6)),
                        ));
                        parent.spawn((
                            Text::new("Press [Backspace] to exit and return to overview"),
                            TextFont::from_font_size(18.0),
                            TextColor(Color::srgb(0.7, 0.7, 0.7)),
                        ));
                    });
            },
            despawn_active_toast,
        ),
    );

    let enter_state = room_state.clone();
    let gp_state = gameplay_state.clone();
    app.add_systems(
        Update,
        (move |input: Res<ButtonInput<KeyCode>>,
               player_query: Single<&BuildingEntrance, With<Player>>,
               mut next_screen: ResMut<NextState<S>>| {
            if input.just_pressed(KeyCode::Enter) {
                try_enter_room(
                    &player_query,
                    entrance,
                    enter_state.clone(),
                    &mut next_screen,
                );
            }
        })
        .run_if(in_state(gp_state)),
    );

    let enter_state = room_state.clone();
    let gp_state = gameplay_state.clone();
    app.add_systems(
        Update,
        (move |input: Res<ButtonInput<KeyCode>>,
               player_query: Single<&CurrentTilePosition, With<Player>>,
               mut next_screen: ResMut<NextState<S>>,
               mut spawn_point: ResMut<PlayerSpawnPoint>| {
            let force = input.just_pressed(KeyCode::Backspace);
            try_exit_room(
                player_query.0,
                exit_door,
                exit_spawn,
                gp_state.clone(),
                &mut spawn_point,
                &mut next_screen,
                force,
            );
        })
        .run_if(in_state(enter_state)),
    );
}

fn spawn_interior_map(parent: &mut ChildSpawnerCommands, map_handle: Handle<TiledMapAsset>) {
    parent.spawn((
        Name::new("Interior Map"),
        TiledMap(map_handle),
        TilemapAnchor::BottomLeft,
        Transform::from_xyz(-(TILE_SIZE as f32 / 2.0), -(TILE_SIZE as f32 / 2.0), 0.0),
    ));
}

fn nutrition_house_map(assets: &InteriorAssets) -> Handle<TiledMapAsset> {
    assets.nutrition_house.clone()
}

fn push_pop_enclosure_map(assets: &InteriorAssets) -> Handle<TiledMapAsset> {
    assets.push_pop_enclosure.clone()
}

fn spawn_polly_extras(
    parent: &mut ChildSpawnerCommands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<ColorMaterial>,
    tile: TilePosition,
) {
    spawn_polly_npc(parent, meshes, materials, tile);
}

fn spawn_push_pop_extras(
    parent: &mut ChildSpawnerCommands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<ColorMaterial>,
    tile: TilePosition,
) {
    spawn_push_pop_npc(parent, meshes, materials, tile);
}

pub struct NutritionHousePlugin;

impl Plugin for NutritionHousePlugin {
    fn build(&self, app: &mut App) {
        build_room(
            app,
            RoomConfig {
                room_state: Screen::InRoom(InRoom::NutritionHouse),
                gameplay_state: Screen::Gameplay,
                entrance: BuildingEntrance::NutritionHouse,
                enclosure_id: EnclosureId::NutritionHousePlaypen,
                room_spawn: NUTRITION_HOUSE_ROOM.room_spawn,
                exit_spawn: NUTRITION_HOUSE_ROOM.exit_spawn,
                exit_door: NUTRITION_HOUSE_ROOM.exit_door,
                get_interior_map: nutrition_house_map,
                resident_animal: Some(AnimalId::Polly),
                spawn_extras_fn: spawn_polly_extras,
                room_title: "Nutrition House".to_string(),
            },
        );
    }
}

pub struct PushPopEnclosurePlugin;

impl Plugin for PushPopEnclosurePlugin {
    fn build(&self, app: &mut App) {
        build_room(
            app,
            RoomConfig {
                room_state: Screen::InRoom(InRoom::PushPopEnclosure),
                gameplay_state: Screen::Gameplay,
                entrance: BuildingEntrance::PushPopEnclosure,
                enclosure_id: EnclosureId::PushPopEnclosure,
                room_spawn: PUSH_POP_ENCLOSURE_ROOM.room_spawn,
                exit_spawn: PUSH_POP_ENCLOSURE_ROOM.exit_spawn,
                exit_door: PUSH_POP_ENCLOSURE_ROOM.exit_door,
                get_interior_map: push_pop_enclosure_map,
                resident_animal: Some(AnimalId::PushPop),
                spawn_extras_fn: spawn_push_pop_extras,
                room_title: "Push Pop Enclosure".to_string(),
            },
        );
    }
}
