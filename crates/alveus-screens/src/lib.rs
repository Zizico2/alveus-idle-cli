//! The game's main screen states and transitions between them.

pub mod gameplay;
pub mod loading;
mod splash;
mod title;

use alveus_app::Screen;
use alveus_asset_tracking::ResourceHandles;
use alveus_collision::{CollisionMasks, collision_ready};
use bevy::prelude::*;

pub use loading::LoadingTiming;

/// Start a session from the title screen without coupling input or menu UI to
/// asset-loading details.
pub fn begin_play_in_world(world: &mut World) {
    let resources_ready = world.resource::<ResourceHandles>().is_all_done();
    let collision_ready = world
        .get_resource::<CollisionMasks>()
        .is_none_or(collision_ready);
    world
        .resource_mut::<NextState<Screen>>()
        .set(if resources_ready && collision_ready {
            Screen::Gameplay
        } else {
            Screen::Loading
        });
}

/// Adds screen UI and transitions.
///
/// Requires [`alveus_app::plugin`] to initialize the app-wide states first.
pub fn plugin(app: &mut App) {
    app.add_plugins((
        gameplay::plugin,
        loading::plugin,
        splash::plugin,
        title::plugin,
    ));
}
