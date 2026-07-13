//! The pause menu.

use bevy::prelude::*;
use bevy::ui_widgets::Activate;

use alveus_app::Menu;
use alveus_command::GameCommand;
use alveus_theme::widget;

pub(super) fn plugin(app: &mut App) {
    app.add_systems(OnEnter(Menu::Pause), spawn_pause_menu);
}

fn spawn_pause_menu(mut commands: Commands) {
    commands.spawn((
        widget::ui_root("Pause Menu"),
        GlobalZIndex(2),
        DespawnOnExit(Menu::Pause),
        children![
            widget::header("Game paused"),
            widget::button_autofocus("Continue", close_menu),
            widget::button("Settings", open_settings_menu),
            widget::button("Quit to title", quit_to_title),
        ],
    ));
}

fn open_settings_menu(_: On<Activate>, mut commands: Commands) {
    commands.trigger(GameCommand::OpenSettings);
}

fn close_menu(_: On<Activate>, mut commands: Commands) {
    commands.trigger(GameCommand::Continue);
}

fn quit_to_title(_: On<Activate>, mut commands: Commands) {
    commands.trigger(GameCommand::QuitToTitle);
}
