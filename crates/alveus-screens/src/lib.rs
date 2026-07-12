//! The game's main screen states and transitions between them.

pub mod gameplay;
pub mod loading;
mod splash;
mod title;

use bevy::prelude::*;

pub use loading::LoadingTiming;

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
