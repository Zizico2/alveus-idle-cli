//! Development tools for the game. This plugin is only enabled in dev builds.

use bevy::{dev_tools::states::log_transitions, prelude::*};

use alveus_app::Screen;
use alveus_input::ToggleUiDebug;

pub(super) fn plugin(app: &mut App) {
    // Log `Screen` state transitions.
    app.add_systems(Update, log_transitions::<Screen>);

    app.add_observer(toggle_debug_ui);
}

fn toggle_debug_ui(_: On<ToggleUiDebug>, mut options: ResMut<GlobalUiDebugOptions>) {
    options.toggle();
}
