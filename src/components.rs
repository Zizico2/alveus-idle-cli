use bevy::prelude::*;

#[derive(Component, Debug, Reflect, Default, Clone, Copy)]
#[reflect(Component, Default)]
pub enum BuildingEntrance {
    #[default]
    NoEntrance,
    NutritionHouse,
}