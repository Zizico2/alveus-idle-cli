//! Overview & interior world: level loading, player bundle, tile-based movement,
//! room transitions, building entrances, camera follow, and toast notifications.

use bevy::prelude::*;
use bevy_tweening::TweeningPlugin;

pub mod camera;
pub mod entrance;
pub mod interiors;
pub mod level;
pub mod movement;
pub mod player;
pub mod room;
pub mod toast;

pub fn plugin(app: &mut App) {
    app.add_plugins((
        TweeningPlugin,
        camera::CameraControllerPlugin,
        level::plugin,
        movement::plugin,
        toast::ToastPlugin,
        entrance::EntrancePlugin,
        interiors::plugin,
        room::NutritionHousePlugin,
        room::PushPopEnclosurePlugin,
    ));
}
