//! The main menu (seen on the title screen).

use bevy::prelude::*;
use bevy::ui_widgets::Activate;

use alveus_app::Menu;
use alveus_command::GameCommand;
use alveus_theme::widget;

pub(super) fn plugin(app: &mut App) {
    app.add_systems(OnEnter(Menu::Main), spawn_main_menu);
}

fn spawn_main_menu(mut commands: Commands) {
    commands.spawn((
        widget::ui_root("Main Menu"),
        GlobalZIndex(2),
        DespawnOnExit(Menu::Main),
        #[cfg(not(target_family = "wasm"))]
        children![
            widget::button_autofocus("Play", enter_loading_or_gameplay_screen),
            widget::button("Settings", open_settings_menu),
            widget::button("Credits", open_credits_menu),
            widget::button("Exit", exit_app),
        ],
        #[cfg(target_family = "wasm")]
        children![
            widget::button_autofocus("Play", enter_loading_or_gameplay_screen),
            widget::button("Settings", open_settings_menu),
            widget::button("Credits", open_credits_menu),
        ],
    ));
}

fn enter_loading_or_gameplay_screen(_: On<Activate>, mut commands: Commands) {
    commands.trigger(GameCommand::Play);
}

fn open_settings_menu(_: On<Activate>, mut commands: Commands) {
    commands.trigger(GameCommand::OpenSettings);
}

fn open_credits_menu(_: On<Activate>, mut commands: Commands) {
    commands.trigger(GameCommand::OpenCredits);
}

#[cfg(not(target_family = "wasm"))]
fn exit_app(_: On<Activate>, mut app_exit: MessageWriter<AppExit>) {
    app_exit.write(AppExit::Success);
}
