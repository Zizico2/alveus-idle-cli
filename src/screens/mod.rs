//! The game's main screen states and transitions between them.

pub mod gameplay;
mod loading;
mod splash;
mod title;

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

#[derive(Clone, Copy, Eq, PartialEq, Hash, Debug, Reflect)]
pub enum InRoom {
    NutritionHouse,
    PushPopEnclosure,
}

/// The game's main screen states.
#[derive(States, Copy, Clone, Eq, PartialEq, Hash, Debug, Default, Reflect)]
pub enum Screen {
    #[default]
    Splash,
    Title,
    Loading,
    Gameplay,
    InRoom(InRoom),
}
