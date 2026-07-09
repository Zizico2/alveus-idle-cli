//! Player bundle construction. The `Player` marker and movement components live
//! in `alveus_components`; keyboard input mapping lives in `alveus_headless`.

use bevy::prelude::*;

use alveus_components::{DynamicObstacle, MovementController, Player, TILE_SIZE, TilePosition};

pub const PLAYER_Z_INDEX: f32 = 2.0;

/// The player character bundle.
pub fn player(
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<ColorMaterial>,
    spawn_pos: TilePosition,
) -> impl Bundle {
    (
        Name::new("Player"),
        Player,
        DynamicObstacle,
        Mesh2d(meshes.add(Circle::new(16.))),
        MeshMaterial2d(materials.add(Color::srgb(0.3, 0.1, 0.9))),
        Transform::from_xyz(
            spawn_pos.x as f32 * TILE_SIZE as f32,
            spawn_pos.y as f32 * TILE_SIZE as f32,
            PLAYER_Z_INDEX,
        ),
        MovementController::default(),
    )
}
