//! The title screen that appears after the splash screen.

use bevy::prelude::*;

use alveus_app::{Menu, Screen};

use crate::loading::surface_loading_diagnostic_on_title;

pub(super) fn plugin(app: &mut App) {
    app.add_systems(
        OnEnter(Screen::Title),
        (open_main_menu, surface_loading_diagnostic_on_title),
    );
    app.add_systems(OnExit(Screen::Title), close_menu);
}

fn open_main_menu(mut next_menu: ResMut<NextState<Menu>>) {
    next_menu.set(Menu::Main);
}

fn close_menu(mut next_menu: ResMut<NextState<Menu>>) {
    next_menu.set(Menu::None);
}
