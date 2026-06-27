use bevy::prelude::*;

#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Reflect)]
#[reflect(Component, Default)]
pub struct TilePosition {
    pub x: u32,
    pub y: u32,
}

#[derive(Clone, Copy, Debug, Default, Component, PartialEq, Eq, Reflect)]
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
pub struct InEnclosure(pub crate::stats::EnclosureId);

/// Save-backed dynamic tile synced to [`crate::collision::DynamicObstacleTiles`]
/// while the entity is loaded (e.g. manure piles spawned in-room).
#[derive(Component, Debug, Reflect, Default, Clone, Copy, PartialEq, Eq)]
#[reflect(Component, Default)]
pub struct PersistedDynamicObstacle;