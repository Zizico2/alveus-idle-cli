//! Reusable UI widgets & theming.

pub mod interaction;
pub mod palette;
pub mod widget;

#[allow(unused_imports)]
pub mod prelude {
    pub use super::{interaction::InteractionPalette, palette as ui_palette, widget};
}

use bevy::prelude::*;

pub fn plugin(app: &mut App) {
    app.add_plugins(interaction::plugin);
}
