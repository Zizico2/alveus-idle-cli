//! The game's main screen states and transitions between them.

pub mod gameplay;
pub mod loading;
mod splash;
mod title;

use alveus_app::Screen;
use alveus_app::ensure_plugin;
use alveus_asset_tracking::ResourceHandles;
use alveus_collision::{CollisionMasks, collision_ready};
use bevy::prelude::*;

pub use loading::LoadingTiming;

/// Internal command-routing event (not Reflect-registered).
#[doc(hidden)]
#[derive(Event, Debug, Clone, Copy)]
pub struct PlayRequest;

/// Handles [`PlayRequest`] from the command router.
pub struct ScreenCommandHandlersPlugin;

impl Plugin for ScreenCommandHandlersPlugin {
    fn build(&self, app: &mut App) {
        app.add_observer(on_play_request);
    }
}

fn on_play_request(
    _trigger: On<PlayRequest>,
    resource_handles: Res<ResourceHandles>,
    masks: Option<Res<CollisionMasks>>,
    mut next_screen: ResMut<NextState<Screen>>,
) {
    let resources_ready = resource_handles.is_all_done();
    let collision_ready_flag = masks
        .as_ref()
        .map(|masks| collision_ready(masks))
        .unwrap_or(true);
    next_screen.set(if resources_ready && collision_ready_flag {
        Screen::Gameplay
    } else {
        Screen::Loading
    });
}

/// Adds screen UI and transitions.
///
/// Requires [`alveus_app::plugin`] to initialize the app-wide states first.
pub fn plugin(app: &mut App) {
    ensure_plugin(app, ScreenCommandHandlersPlugin);
    app.add_plugins((
        gameplay::plugin,
        loading::plugin,
        splash::plugin,
        title::plugin,
    ));
}
