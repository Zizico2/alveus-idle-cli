//! Keyboard input mapping: translate raw key presses into [`GameCommand`]s.
//!
//! Per the project's golden rules, keyboard handlers are thin and only
//! `trigger(GameCommand::...)`; the same verbs are what external BRP clients
//! send. Keeping this mapping in the command crate centralizes the input →
//! verb boundary and avoids the `world`/`screens` crates depending on the
//! dispatcher.

use bevy::{input::common_conditions::input_just_pressed, prelude::*};

use alveus_app::{AppSystems, Menu, PausableSystems, Screen, tile_interaction_enabled};
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
            (
                record_player_directional_input.run_if(tile_interaction_enabled),
                record_care_picker_navigation.run_if(in_state(Menu::CareItemPicker)),
            )
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
                    .run_if(tile_interaction_enabled.and_then(input_just_pressed(KeyCode::Space))),
                drop_item_from_keyboard
                    .run_if(tile_interaction_enabled.and_then(input_just_pressed(KeyCode::KeyK))),
                confirm_care_menu_from_keyboard.run_if(in_state(Menu::CareItemPicker).and_then(
                    input_just_pressed(KeyCode::Enter).or_else(input_just_pressed(KeyCode::Space)),
                )),
                cancel_care_menu_from_keyboard.run_if(
                    in_state(Menu::CareItemPicker).and_then(input_just_pressed(KeyCode::Escape)),
                ),
            ),
        );
    }
}

fn record_player_directional_input(input: Res<ButtonInput<KeyCode>>, mut commands: Commands) {
    let is_up = input.pressed(KeyCode::KeyW) || input.pressed(KeyCode::ArrowUp);
    let is_down = input.pressed(KeyCode::KeyS) || input.pressed(KeyCode::ArrowDown);
    let is_left = input.pressed(KeyCode::KeyA) || input.pressed(KeyCode::ArrowLeft);
    let is_right = input.pressed(KeyCode::KeyD) || input.pressed(KeyCode::ArrowRight);

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

fn record_care_picker_navigation(input: Res<ButtonInput<KeyCode>>, mut commands: Commands) {
    if input.just_pressed(KeyCode::KeyW) || input.just_pressed(KeyCode::ArrowUp) {
        commands.trigger(GameCommand::Move(MovementIntent::Up));
    } else if input.just_pressed(KeyCode::KeyS) || input.just_pressed(KeyCode::ArrowDown) {
        commands.trigger(GameCommand::Move(MovementIntent::Down));
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
