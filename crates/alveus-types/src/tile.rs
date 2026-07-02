use bevy_ecs::prelude::*;
use bevy_reflect::Reflect;
use bevy_reflect::prelude::ReflectDefault;

#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Reflect)]
#[reflect(Component, Default)]
pub struct TilePosition {
    pub x: u32,
    pub y: u32,
}

/// Inclusive rectangular tile range (bottom-left to top-right).
#[derive(Debug, Clone, Copy)]
pub struct TileBounds {
    pub bottom_left: TilePosition,
    pub top_right: TilePosition,
}
