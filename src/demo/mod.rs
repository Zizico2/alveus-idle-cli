//! Demo gameplay. All of these modules are only intended for demonstration
//! purposes and should be replaced with your own game logic.
//! Feel free to change the logic found here if you feel like tinkering around
//! to get a feeling for the template.

use bevy::prelude::*;

mod animation;
mod camera;
pub mod level;
mod movement;
pub mod player;
pub mod toast;
mod entrance;
pub mod interiors;
pub mod room;

pub(super) fn plugin(app: &mut App) {
    app.add_plugins((
        animation::plugin,
        camera::CameraControllerPlugin,
        level::plugin,
        movement::plugin,
        player::plugin,
        toast::ToastPlugin,
        entrance::EntrancePlugin,
        interiors::plugin,
        room::NutritionHousePlugin,
        room::PushPopEnclosurePlugin,
    ));
}
