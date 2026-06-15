use bevy::prelude::*;
use bevy::state::state::FreelyMutableState;
use crate::animals::spawn_push_pop_npc;
use crate::components::{CurrentTilePosition, DesiredTilePosition, Obstacle, TilePosition, BuildingEntrance};
use crate::content::{
    PUSH_POP_ENCLOSURE_OBJECTS, PUSH_POP_PLACEMENT, NUTRITION_HOUSE_OBJECTS, RoomObjectDef,
};
use crate::demo::player::{player, Player, PlayerAssets};
use crate::demo::toast::despawn_active_toast;
use crate::interaction::Interactable;
use crate::screens::{Screen, InRoom};
use crate::demo::level::TILE_SIZE;

#[derive(Resource, Debug, Clone, Copy)]
pub struct PlayerSpawnPoint {
    pub position: TilePosition,
}

impl Default for PlayerSpawnPoint {
    fn default() -> Self {
        Self {
            // Default starting position when entering the game
            position: TilePosition { x: 0, y: 0 },
        }
    }
}

pub struct RoomConfig<S: States + FreelyMutableState> {
    pub room_state: S,
    pub gameplay_state: S,
    pub entrance: BuildingEntrance,
    pub room_spawn: TilePosition,
    pub exit_spawn: TilePosition,
    pub exit_door: TilePosition,
    pub spawn_interior_fn: fn(&mut ChildSpawnerCommands, &mut Assets<Mesh>, &mut Assets<ColorMaterial>),
    pub room_title: String,
}

pub fn build_room<S: States + FreelyMutableState>(app: &mut App, config: RoomConfig<S>) {
    app.init_resource::<PlayerSpawnPoint>();

    let room_state = config.room_state.clone();
    let gameplay_state = config.gameplay_state.clone();
    let entrance = config.entrance;
    let room_spawn = config.room_spawn;
    let exit_spawn = config.exit_spawn;
    let exit_door = config.exit_door;
    let spawn_interior_fn = config.spawn_interior_fn;
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
                  player_assets: Res<PlayerAssets>,
                  mut texture_atlas_layouts: ResMut<Assets<TextureAtlasLayout>>,
                  mut meshes: ResMut<Assets<Mesh>>,
                  mut materials: ResMut<Assets<ColorMaterial>>| {
                
                commands.spawn((
                    Name::new(format!("{} Room", room_title_for_spawner)),
                    Transform::default(),
                    Visibility::default(),
                    DespawnOnExit(enter_state_for_spawner.clone()),
                )).with_children(|parent| {
                    // Spawn Player
                    parent.spawn((
                        player(
                            400.0,
                            &player_assets,
                            &mut texture_atlas_layouts,
                            &mut meshes,
                            &mut materials,
                            room_spawn,
                        ),
                        CurrentTilePosition(room_spawn),
                        DesiredTilePosition(room_spawn),
                    ));

                    // Call the custom spawner for this room's interior elements
                    spawn_interior_fn(parent, &mut meshes, &mut materials);
                });
            },
            // Spawn Room UI overlay
            move |mut commands: Commands| {
                commands.spawn((
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
                )).with_children(|parent| {
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
        )
    );

    // Update enter transitions
    let enter_state = room_state.clone();
    let gp_state = gameplay_state.clone();
    app.add_systems(
        Update,
        (move |input: Res<ButtonInput<KeyCode>>,
              player_query: Single<&BuildingEntrance, With<Player>>,
              mut next_screen: ResMut<NextState<S>>| {
            if input.just_pressed(KeyCode::Enter) && *player_query == &entrance {
                info!("Entering room interior state!");
                next_screen.set(enter_state.clone());
            }
        })
        .run_if(in_state(gp_state)),
    );

    // Update exit transitions
    let enter_state = room_state.clone();
    let gp_state = gameplay_state.clone();
    app.add_systems(
        Update,
        (move |input: Res<ButtonInput<KeyCode>>,
              player_query: Single<&CurrentTilePosition, With<Player>>,
              mut next_screen: ResMut<NextState<S>>,
              mut spawn_point: ResMut<PlayerSpawnPoint>| {
            let mut should_exit = false;

            if input.just_pressed(KeyCode::Backspace) {
                should_exit = true;
            }

            if player_query.0.x == exit_door.x && player_query.0.y == exit_door.y {
                should_exit = true;
            }

            if should_exit {
                info!("Exiting room interior!");
                spawn_point.position = exit_spawn;
                next_screen.set(gp_state.clone());
            }
        })
        .run_if(in_state(enter_state)),
    );
}

// ============================================
// Nutrition House Room Specific Implementation
// ============================================

pub struct NutritionHousePlugin;

impl Plugin for NutritionHousePlugin {
    fn build(&self, app: &mut App) {
        build_room(app, RoomConfig {
            room_state: Screen::InRoom(InRoom::NutritionHouse),
            gameplay_state: Screen::Gameplay,
            entrance: BuildingEntrance::NutritionHouse,
            room_spawn: TilePosition { x: 5, y: 2 },
            exit_spawn: TilePosition { x: 33, y: 12 },
            exit_door: TilePosition { x: 5, y: 0 },
            spawn_interior_fn: spawn_nutrition_house_interior,
            room_title: "Nutrition House".to_string(),
        });
    }
}

pub struct PushPopEnclosurePlugin;

impl Plugin for PushPopEnclosurePlugin {
    fn build(&self, app: &mut App) {
        build_room(app, RoomConfig {
            room_state: Screen::InRoom(InRoom::PushPopEnclosure),
            gameplay_state: Screen::Gameplay,
            entrance: BuildingEntrance::PushPopEnclosure,
            room_spawn: TilePosition { x: 6, y: 2 },
            exit_spawn: TilePosition { x: 40, y: 33 },
            exit_door: TilePosition { x: 6, y: 0 },
            spawn_interior_fn: spawn_push_pop_enclosure_interior,
            room_title: "Push Pop Enclosure".to_string(),
        });
    }
}

fn spawn_room_object(
    parent: &mut ChildSpawnerCommands,
    _meshes: &mut Assets<Mesh>,
    materials: &mut Assets<ColorMaterial>,
    wall_mesh: &Handle<Mesh>,
    object: &RoomObjectDef,
) {
    let material = materials.add(object.color);
    let mut entity = parent.spawn((
        Name::new(object.display_name),
        Mesh2d(wall_mesh.clone()),
        MeshMaterial2d(material),
        Transform::from_xyz(
            object.position.x as f32 * TILE_SIZE as f32,
            object.position.y as f32 * TILE_SIZE as f32,
            0.2,
        ),
        TilePosition {
            x: object.position.x,
            y: object.position.y,
        },
    ));

    if object.is_obstacle {
        entity.insert(Obstacle);
    }

    if let Some(interaction) = object.interaction {
        entity.insert(Interactable {
            object_id: object.object_id,
            position: object.position,
            interaction,
        });
    }
}

fn spawn_nutrition_house_interior(
    parent: &mut ChildSpawnerCommands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<ColorMaterial>,
) {
    // Floor: x: 1..=9, y: 1..=9
    let floor_color = Color::srgb(0.22, 0.16, 0.12);
    let floor_material = materials.add(floor_color);
    let floor_mesh = meshes.add(Rectangle::new(30.0, 30.0));
    for x in 1..=9 {
        for y in 1..=9 {
            parent.spawn((
                Name::new(format!("Floor Tile ({}, {})", x, y)),
                Mesh2d(floor_mesh.clone()),
                MeshMaterial2d(floor_material.clone()),
                Transform::from_xyz(
                    x as f32 * TILE_SIZE as f32,
                    y as f32 * TILE_SIZE as f32,
                    0.0,
                ),
            ));
        }
    }

    // Door: x: 5, y: 0
    let door_color = Color::srgb(0.4, 0.25, 0.15);
    parent.spawn((
        Name::new("Door Tile"),
        Mesh2d(meshes.add(Rectangle::new(30.0, 30.0))),
        MeshMaterial2d(materials.add(door_color)),
        Transform::from_xyz(
            5.0 * TILE_SIZE as f32,
            0.0 * TILE_SIZE as f32,
            0.01,
        ),
        TilePosition { x: 5, y: 0 },
    ));

    // Walls
    let wall_color = Color::srgb(0.25, 0.28, 0.3);
    let wall_material = materials.add(wall_color);
    let wall_mesh = meshes.add(Rectangle::new(30.0, 30.0));

    let spawn_wall = |p: &mut ChildSpawnerCommands, x: u32, y: u32| {
        p.spawn((
            Name::new(format!("Wall Tile ({}, {})", x, y)),
            Mesh2d(wall_mesh.clone()),
            MeshMaterial2d(wall_material.clone()),
            Transform::from_xyz(
                x as f32 * TILE_SIZE as f32,
                y as f32 * TILE_SIZE as f32,
                0.1,
            ),
            TilePosition { x, y },
            Obstacle,
        ));
    };

    // Bottom wall (excluding x=5)
    for x in 0..=10 {
        if x != 5 {
            spawn_wall(parent, x, 0);
        }
    }
    // Top wall
    for x in 0..=10 {
        spawn_wall(parent, x, 10);
    }
    // Left wall
    for y in 1..=9 {
        spawn_wall(parent, 0, y);
    }
    // Right wall
    for y in 1..=9 {
        spawn_wall(parent, 10, y);
    }

    // Prep Table (x: 4..=6, y: 7)
    let counter_color = Color::srgb(0.1, 0.5, 0.4);
    let counter_material = materials.add(counter_color);
    for x in 4..=6 {
        parent.spawn((
            Name::new(format!("Prep Table ({}, 7)", x)),
            Mesh2d(wall_mesh.clone()),
            MeshMaterial2d(counter_material.clone()),
            Transform::from_xyz(
                x as f32 * TILE_SIZE as f32,
                7.0 * TILE_SIZE as f32,
                0.2,
            ),
            TilePosition { x, y: 7 },
            Obstacle,
        ));
    }

    for object in NUTRITION_HOUSE_OBJECTS {
        spawn_room_object(parent, meshes, materials, &wall_mesh, object);
    }
}

fn spawn_push_pop_enclosure_interior(
    parent: &mut ChildSpawnerCommands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<ColorMaterial>,
) {
    let floor_color = Color::srgb(0.72, 0.62, 0.45);
    let floor_material = materials.add(floor_color);
    let floor_mesh = meshes.add(Rectangle::new(30.0, 30.0));
    for x in 1..=11 {
        for y in 1..=11 {
            parent.spawn((
                Name::new(format!("Floor Tile ({}, {})", x, y)),
                Mesh2d(floor_mesh.clone()),
                MeshMaterial2d(floor_material.clone()),
                Transform::from_xyz(
                    x as f32 * TILE_SIZE as f32,
                    y as f32 * TILE_SIZE as f32,
                    0.0,
                ),
            ));
        }
    }

    let door_color = Color::srgb(0.50, 0.40, 0.28);
    parent.spawn((
        Name::new("Door Tile"),
        Mesh2d(meshes.add(Rectangle::new(30.0, 30.0))),
        MeshMaterial2d(materials.add(door_color)),
        Transform::from_xyz(
            6.0 * TILE_SIZE as f32,
            0.0 * TILE_SIZE as f32,
            0.01,
        ),
        TilePosition { x: 6, y: 0 },
    ));

    let wall_color = Color::srgb(0.45, 0.38, 0.28);
    let wall_material = materials.add(wall_color);
    let wall_mesh = meshes.add(Rectangle::new(30.0, 30.0));

    let spawn_wall = |p: &mut ChildSpawnerCommands, x: u32, y: u32| {
        p.spawn((
            Name::new(format!("Fence ({}, {})", x, y)),
            Mesh2d(wall_mesh.clone()),
            MeshMaterial2d(wall_material.clone()),
            Transform::from_xyz(
                x as f32 * TILE_SIZE as f32,
                y as f32 * TILE_SIZE as f32,
                0.1,
            ),
            TilePosition { x, y },
            Obstacle,
        ));
    };

    for x in 0..=12 {
        if x != 6 {
            spawn_wall(parent, x, 0);
        }
    }
    for x in 0..=12 {
        spawn_wall(parent, x, 12);
    }
    for y in 1..=11 {
        spawn_wall(parent, 0, y);
    }
    for y in 1..=11 {
        spawn_wall(parent, 12, y);
    }

    // Shelter (3,9) 2x2
    let shelter_color = Color::srgb(0.40, 0.32, 0.22);
    let shelter_material = materials.add(shelter_color);
    for x in 3..=4 {
        for y in 9..=10 {
            parent.spawn((
                Name::new(format!("Shelter ({}, {})", x, y)),
                Mesh2d(wall_mesh.clone()),
                MeshMaterial2d(shelter_material.clone()),
                Transform::from_xyz(
                    x as f32 * TILE_SIZE as f32,
                    y as f32 * TILE_SIZE as f32,
                    0.2,
                ),
                TilePosition { x, y },
                Obstacle,
            ));
        }
    }

    for object in PUSH_POP_ENCLOSURE_OBJECTS {
        spawn_room_object(parent, meshes, materials, &wall_mesh, object);
    }

    spawn_push_pop_npc(
        parent,
        meshes,
        materials,
        PUSH_POP_PLACEMENT.home_position,
    );
}
