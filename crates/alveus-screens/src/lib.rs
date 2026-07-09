//! The game's main screen states and transitions between them.

pub mod gameplay;
mod loading;
mod splash;
mod title;

use alveus_app::Screen;
use bevy::prelude::*;

pub fn plugin(app: &mut App) {
    app.init_state::<Screen>();

    app.add_plugins((
        gameplay::plugin,
        loading::plugin,
        splash::plugin,
        title::plugin,
    ));
}
