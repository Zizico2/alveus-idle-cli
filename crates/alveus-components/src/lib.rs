//! Shared ECS components, markers, and cross-feature state.
//!
//! This crate sits just above [`alveus_types`] and holds the data that multiple
//! feature crates need in common (spatial components, the player marker, the
//! movement controller, and small shared resources). Behaviour lives in the
//! feature crates; this crate is deliberately data-only.

use bevy::prelude::*;

pub use alveus_configs::{PLAYER_MOVE_DURATION_SECS, TILE_SIZE};
pub use alveus_types::{EnclosureId, TilePosition};

// ---------------------------------------------------------
// Spatial components
// ---------------------------------------------------------

#[derive(Clone, Copy, Debug, Default, Component, PartialEq, Eq, Reflect)]
#[reflect(Component)]
pub struct CurrentTilePosition(pub TilePosition);

#[derive(Clone, Copy, Debug, Default, Component, PartialEq, Eq, Reflect)]
pub struct DesiredTilePosition(pub TilePosition);

#[derive(Component, Debug, Clone, Reflect)]
pub enum TileGroup {
    Rectangle(RectangleTileGroup),
}

#[derive(Debug, Clone, Reflect)]
pub struct RectangleTileGroup {
    pub bottom_left: TilePosition,
    pub top_right: TilePosition,
}

#[derive(Component, Debug, Reflect, Default, Clone, Copy, PartialEq, Eq)]
#[reflect(Component, Default)]
pub enum BuildingEntrance {
    #[default]
    NoEntrance,
    NutritionHouse,
    PushPopEnclosure,
}

#[derive(Component, Debug, Reflect, Default, Clone, Copy, PartialEq, Eq)]
#[reflect(Component, Default)]
pub struct Obstacle;

/// Blocks movement for other entities. Paired with [`CurrentTilePosition`];
/// queried live each frame (not saved).
///
/// Scope is determined by [`InEnclosure`]: entities without it block on every
/// collision map (e.g. the player); entities with it block only in that enclosure.
#[derive(Component, Debug, Reflect, Default, Clone, Copy, PartialEq, Eq)]
#[reflect(Component, Default)]
pub struct DynamicObstacle;

/// Limits a [`DynamicObstacle`] (or other enclosure-scoped logic) to one interior map.
#[derive(Component, Debug, Reflect, Clone, Copy, PartialEq, Eq)]
#[reflect(Component)]
pub struct InEnclosure(pub EnclosureId);

/// Save-backed dynamic tile synced to `DynamicObstacleTiles` while the entity is
/// loaded (e.g. poop piles in Push Pop).
#[derive(Component, Debug, Reflect, Default, Clone, Copy, PartialEq, Eq)]
#[reflect(Component, Default)]
pub struct PersistedDynamicObstacle;

// ---------------------------------------------------------
// Interaction marker
// ---------------------------------------------------------

/// Marker for tiles the player can interact with when adjacent.
#[derive(Component, Debug, Clone, Copy, Reflect, Default)]
#[reflect(Component, Default)]
pub struct Interactable;

// ---------------------------------------------------------
// Player & movement
// ---------------------------------------------------------

/// The player character marker.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Default, Reflect)]
#[reflect(Component)]
#[require(
    CurrentTilePosition,
    DesiredTilePosition,
    MovementDuration(Timer::from_seconds(PLAYER_MOVE_DURATION_SECS, TimerMode::Once))
)]
pub struct Player;

impl PartialEq<CurrentTilePosition> for DesiredTilePosition {
    fn eq(&self, other: &CurrentTilePosition) -> bool {
        self.0 == other.0
    }
}
impl PartialEq<DesiredTilePosition> for CurrentTilePosition {
    fn eq(&self, other: &DesiredTilePosition) -> bool {
        self.0 == other.0
    }
}

/// Movement parameters for the tile-based character controller.
#[derive(Component, Reflect, Default)]
#[reflect(Component, Default)]
pub struct MovementController {
    /// The direction the character wants to move in.
    pub intent: Option<MovementIntent>,
}

#[derive(Reflect, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum MovementIntent {
    Up,
    Down,
    Left,
    Right,
}

/// Duration of a single tile step.
#[derive(Component)]
pub struct MovementDuration(pub Timer);

// ---------------------------------------------------------
// Shared cross-feature resources
// ---------------------------------------------------------

/// Poops carried in the wheelbarrow, in pickup order (max `WHEELBARROW_CAPACITY`).
#[derive(Resource, Debug, Clone, Default, Reflect)]
#[reflect(Resource)]
pub struct PoopWheelbarrow {
    pub poops: Vec<EnclosureId>,
}

impl PoopWheelbarrow {
    pub fn count(&self) -> u8 {
        self.poops.len().min(u8::MAX as usize) as u8
    }
}

/// Transient feedback message surfaced in the HUD after pickups/drops.
#[derive(Resource, Debug, Default, Reflect)]
#[reflect(Resource)]
pub struct LastPickupMessage {
    pub text: Option<String>,
    pub timer: Timer,
}
