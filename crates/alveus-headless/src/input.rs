//! Keyboard input mapping: translate raw key presses into [`GameCommand`]s.
//!
//! Per the project's golden rules, keyboard handlers are thin and only
//! `trigger(GameCommand::...)`; the same verbs are what external BRP clients
//! send. Keeping this mapping in the command crate centralizes the input →
//! verb boundary and avoids the `world`/`screens` crates depending on the
//! dispatcher.

use bevy::{input::common_conditions::input_just_pressed, prelude::*};

use alveus_app::{AppSystems, Menu, PausableSystems, Screen};
use alveus_components::MovementIntent;

use crate::command::GameCommand;

/// Registers the keyboard readers that map key presses to [`GameCommand`]s.
///
/// Added by the game binary for both windowed and headless play. Not added by
/// the minimal test harnesses, which trigger verbs directly.
pub struct InputPlugin;

impl Plugin for InputPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            record_player_directional_input
                .in_set(AppSystems::RecordInput)
                .in_set(PausableSystems),
        );

        app.add_systems(
            Update,
            skip_splash_from_keyboard
                .run_if(input_just_pressed(KeyCode::Escape).and_then(in_state(Screen::Splash))),
        );

        app.add_systems(
            Update,
            pause_from_keyboard.run_if(
                in_state(Screen::Gameplay)
                    .and_then(in_state(Menu::None))
                    .and_then(
                        input_just_pressed(KeyCode::KeyP)
                            .or_else(input_just_pressed(KeyCode::Escape)),
                    ),
            ),
        );

        app.add_systems(
            Update,
            close_menu_from_keyboard.run_if(
                in_state(Screen::Gameplay)
                    .and_then(not(in_state(Menu::None)))
                    .and_then(input_just_pressed(KeyCode::KeyP)),
            ),
        );

        app.add_systems(
            Update,
            (
                interact_from_keyboard
                    .run_if(allows_tile_interaction.and_then(input_just_pressed(KeyCode::Space))),
                drop_item_from_keyboard
                    .run_if(allows_tile_interaction.and_then(input_just_pressed(KeyCode::KeyK))),
                confirm_care_menu_from_keyboard.run_if(
                    in_state(Menu::CareItemPicker).and_then(input_just_pressed(KeyCode::Enter)),
                ),
                cancel_care_menu_from_keyboard.run_if(
                    in_state(Menu::CareItemPicker).and_then(input_just_pressed(KeyCode::Escape)),
                ),
            ),
        );
    }
}

fn allows_tile_interaction(screen: Res<State<Screen>>) -> bool {
    matches!(*screen.get(), Screen::Gameplay | Screen::InRoom(_))
}

fn record_player_directional_input(
    input: Res<ButtonInput<KeyCode>>,
    menu: Res<State<Menu>>,
    mut commands: Commands,
) {
    let picker_open = *menu.get() == Menu::CareItemPicker;
    let is_up = if picker_open {
        input.just_pressed(KeyCode::KeyW) || input.just_pressed(KeyCode::ArrowUp)
    } else {
        input.pressed(KeyCode::KeyW) || input.pressed(KeyCode::ArrowUp)
    };
    let is_down = if picker_open {
        input.just_pressed(KeyCode::KeyS) || input.just_pressed(KeyCode::ArrowDown)
    } else {
        input.pressed(KeyCode::KeyS) || input.pressed(KeyCode::ArrowDown)
    };
    let is_left = if picker_open {
        input.just_pressed(KeyCode::KeyA) || input.just_pressed(KeyCode::ArrowLeft)
    } else {
        input.pressed(KeyCode::KeyA) || input.pressed(KeyCode::ArrowLeft)
    };
    let is_right = if picker_open {
        input.just_pressed(KeyCode::KeyD) || input.just_pressed(KeyCode::ArrowRight)
    } else {
        input.pressed(KeyCode::KeyD) || input.pressed(KeyCode::ArrowRight)
    };

    let intent = if is_up {
        Some(MovementIntent::Up)
    } else if is_down {
        Some(MovementIntent::Down)
    } else if is_left {
        Some(MovementIntent::Left)
    } else if is_right {
        Some(MovementIntent::Right)
    } else {
        None
    };

    if let Some(intent) = intent {
        commands.trigger(GameCommand::Move(intent));
    } else {
        commands.trigger(GameCommand::MoveStop);
    }
}

fn skip_splash_from_keyboard(mut commands: Commands) {
    commands.trigger(GameCommand::SkipSplash);
}

fn pause_from_keyboard(mut commands: Commands) {
    commands.trigger(GameCommand::PauseToggle);
}

fn close_menu_from_keyboard(mut commands: Commands) {
    commands.trigger(GameCommand::PauseToggle);
}

fn interact_from_keyboard(mut commands: Commands) {
    commands.trigger(GameCommand::Interact);
}

fn confirm_care_menu_from_keyboard(mut commands: Commands) {
    commands.trigger(GameCommand::Continue);
}

fn cancel_care_menu_from_keyboard(mut commands: Commands) {
    commands.trigger(GameCommand::Back);
}

fn drop_item_from_keyboard(mut commands: Commands) {
    commands.trigger(GameCommand::DropItem);
}
