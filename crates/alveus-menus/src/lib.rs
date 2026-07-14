//! The game's menus and transitions between them.

pub mod care_item_picker;
mod credits;
mod list_menu;
pub mod main_menu;
mod overlay_menu;
mod pause;
mod settings;
mod standalone_menu;

use bevy::prelude::*;

/// Adds menu UI and transitions.
///
/// Requires [`alveus_app::plugin`] to initialize the app-wide states first.
pub fn plugin(app: &mut App) {
    app.add_plugins(list_menu::ListMenuPlugin);
    app.add_plugins((
        credits::plugin,
        care_item_picker::plugin,
        main_menu::plugin,
        settings::plugin,
        pause::plugin,
    ));
}
