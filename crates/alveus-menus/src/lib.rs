//! The game's menus and transitions between them.

mod credits;
pub mod main_menu;
pub use main_menu::PlayClickEvent;
mod pause;
mod settings;

use alveus_app::Menu;
use bevy::prelude::*;

pub fn plugin(app: &mut App) {
    if !app.world().contains_resource::<State<Menu>>() {
        app.init_state::<Menu>();
    }

    app.add_plugins((
        credits::plugin,
        main_menu::plugin,
        settings::plugin,
        pause::plugin,
    ));
}
