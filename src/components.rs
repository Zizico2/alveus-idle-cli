use bevy::prelude::*;

#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq, Reflect)]
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
}

#[derive(Component, Debug, Reflect, Default, Clone, Copy, PartialEq, Eq)]
#[reflect(Component, Default)]
pub struct Obstacle;