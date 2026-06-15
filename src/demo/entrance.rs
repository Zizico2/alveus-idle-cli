use bevy::prelude::*;
use bevy_ecs_tiled::prelude::*;
use crate::components::{CurrentTilePosition, TileGroup, RectangleTileGroup, TilePosition, BuildingEntrance};
use crate::demo::player::Player;

// Grid Snapping bounds
pub const TILE_SIZE: f32 = 32.0;
pub const GRID_SNAP_EPSILON: f32 = 0.05;

pub struct EntrancePlugin;

impl Plugin for EntrancePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                validate_and_snap_entrances,
                check_player_entrance_transitions,
            ),
        );
    }
}

#[derive(Event, Debug, Clone, Copy)]
pub struct PlayerEnteredBuildingEvent {
    pub entrance: BuildingEntrance,
}

#[derive(Event, Debug, Clone, Copy)]
pub struct PlayerExitedBuildingEvent {
    pub entrance: BuildingEntrance,
}

fn validate_and_snap_entrances(
    mut commands: Commands,
    query: Query<
        (Entity, &Transform, &BuildingEntrance, &TiledObject),
        (Added<BuildingEntrance>, Without<TileGroup>),
    >,
) {
    for (entity, transform, entrance, tiled_object) in query.iter() {
        let x = transform.translation.x;
        let y = transform.translation.y;

        let rem_x = x.rem_euclid(TILE_SIZE);
        let rem_y = y.rem_euclid(TILE_SIZE);

        let dist_x = rem_x.min(TILE_SIZE - rem_x);
        let dist_y = rem_y.min(TILE_SIZE - rem_y);

        if dist_x >= GRID_SNAP_EPSILON || dist_y >= GRID_SNAP_EPSILON {
            panic!(
                "\n❌ MAP INTEGRITY ERROR ❌\nObject: '{:?}'\nPosition: [x:{:.2}, y:{:.2}]\nIssue: Not aligned to {}-pixel grid.\n",
                entrance, x, y, TILE_SIZE
            );
        }

        let TiledObject::Rectangle { width, height } = tiled_object else {
            panic!(
                "\n❌ MAP INTEGRITY ERROR ❌\nObject: '{:?}'\nIssue: Unsupported TiledObject type for size validation.\n",
                entrance
            );
        };

        if width % TILE_SIZE != 0.0 || height % TILE_SIZE != 0.0 {
            panic!(
                "\n❌ MAP INTEGRITY ERROR ❌\nObject: '{:?}'\nSize: [w:{}, h:{}]\nIssue: Dimensions are not multiples of tile size ({}).\n",
                entrance, width, height, TILE_SIZE
            );
        }

        let adjusted_y = y - height;

        let start_grid_x = (x / TILE_SIZE).round() as u32;
        let start_grid_y = (adjusted_y / TILE_SIZE).round() as u32;

        let width_in_tiles = (width / TILE_SIZE).round() as u32;
        let height_in_tiles = (height / TILE_SIZE).round() as u32;

        let tile_group = TileGroup::Rectangle(RectangleTileGroup {
            bottom_left: TilePosition {
                x: start_grid_x,
                y: start_grid_y,
            },
            top_right: TilePosition {
                x: start_grid_x + width_in_tiles - 1,
                y: start_grid_y + height_in_tiles - 1,
            },
        });
        info!("Inserting snapped TileGroup: {:?}", tile_group);

        commands.entity(entity).insert(tile_group);
    }
}

fn check_player_entrance_transitions(
    mut commands: Commands,
    screen: Res<State<crate::screens::Screen>>,
    player_query: Query<(Entity, &CurrentTilePosition, Option<&BuildingEntrance>), With<Player>>,
    entrance_query: Query<(&TileGroup, &BuildingEntrance)>,
) {
    if !matches!(screen.get(), crate::screens::Screen::Gameplay) {
        return;
    }

    let Some((player_entity, player_pos, current_entrance)) = player_query.iter().next() else {
        return;
    };

    let player_grid = player_pos.0;
    let mut overlapping_entrance = None;

    for (tile_group, entrance) in &entrance_query {
        match tile_group {
            TileGroup::Rectangle(rect) => {
                if player_grid.x >= rect.bottom_left.x
                    && player_grid.x <= rect.top_right.x
                    && player_grid.y >= rect.bottom_left.y
                    && player_grid.y <= rect.top_right.y
                {
                    overlapping_entrance = Some(*entrance);
                    break;
                }
            }
        }
    }

    if let Some(entrance) = overlapping_entrance {
        if current_entrance != Some(&entrance) {
            info!("Player entered building entrance: {:?}", entrance);
            commands.trigger(PlayerEnteredBuildingEvent { entrance });
            commands.entity(player_entity).insert(entrance);
        }
    } else {
        if let Some(prev_entrance) = current_entrance {
            info!("Player exited building entrance: {:?}", prev_entrance);
            commands.trigger(PlayerExitedBuildingEvent { entrance: *prev_entrance });
            commands.entity(player_entity).remove::<BuildingEntrance>();
        }
    }
}
