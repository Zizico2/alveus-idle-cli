//! The screen state for the main gameplay.

use bevy::{input::common_conditions::input_just_pressed, prelude::*};

use crate::{Pause, headless::GameCommand, menus::Menu, screens::Screen};

pub(super) fn plugin(app: &mut App) {
    app.add_systems(OnEnter(Screen::Gameplay), crate::demo::level::spawn_level);

    app.add_systems(
        Update,
        pause_from_keyboard.run_if(
            in_state(Screen::Gameplay)
                .and(in_state(Menu::None))
                .and(input_just_pressed(KeyCode::KeyP).or(input_just_pressed(KeyCode::Escape))),
        ),
    );
    app.add_systems(
        Update,
        close_menu_from_keyboard.run_if(
            in_state(Screen::Gameplay)
                .and(not(in_state(Menu::None)))
                .and(input_just_pressed(KeyCode::KeyP)),
        ),
    );
    app.add_systems(OnExit(Screen::Gameplay), (close_menu_state_system, unpause));
    app.add_systems(
        OnEnter(Menu::None),
        unpause.run_if(in_state(Screen::Gameplay)),
    );
}

fn unpause(mut next_pause: ResMut<NextState<Pause>>) {
    next_pause.set(Pause(false));
}

pub fn open_pause_from_gameplay(
    commands: &mut Commands,
    next_pause: &mut NextState<Pause>,
    next_menu: &mut NextState<Menu>,
) {
    next_pause.set(Pause(true));
    spawn_pause_overlay(commands);
    next_menu.set(Menu::Pause);
}

fn pause_from_keyboard(mut commands: Commands) {
    commands.trigger(GameCommand::PauseToggle);
}

pub fn spawn_pause_overlay(commands: &mut Commands) {
    commands.spawn((
        Name::new("Pause Overlay"),
        Node {
            width: percent(100),
            height: percent(100),
            ..default()
        },
        GlobalZIndex(1),
        BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.8)),
        DespawnOnExit(Pause(true)),
    ));
}

pub fn close_menu_state(next_menu: &mut NextState<Menu>) {
    next_menu.set(Menu::None);
}

fn close_menu_from_keyboard(mut commands: Commands) {
    commands.trigger(GameCommand::PauseToggle);
}

fn close_menu_state_system(mut next_menu: ResMut<NextState<Menu>>) {
    close_menu_state(&mut next_menu);
}
