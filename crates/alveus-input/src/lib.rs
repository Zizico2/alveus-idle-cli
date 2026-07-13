//! Keyboard input mapping: translate raw key presses into [`GameCommand`]s.
//!
//! Per the project's golden rules, keyboard handlers are thin and only
//! `trigger(GameCommand::...)`; the same verbs are what external BRP clients
//! send. Keeping this mapping in the input crate centralizes the input →
//! verb boundary and avoids the `world`/`screens` crates depending on the
//! dispatcher.

use bevy::{input::common_conditions::input_just_pressed, prelude::*};

use alveus_app::{AppSystems, Menu, PausableSystems, Screen, tile_interaction_enabled};
use alveus_components::{MovementIntent, Player};

use alveus_command::GameCommand;

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
                record_player_directional_input
                    .run_if(tile_interaction_enabled.and_then(any_with_component::<Player>)),
                record_care_picker_navigation
                    .run_if(in_state(Menu::CareItemPicker).and_then(any_with_component::<Player>)),
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
                interact_from_keyboard.run_if(
                    tile_interaction_enabled
                        .and_then(any_with_component::<Player>)
                        .and_then(input_just_pressed(KeyCode::Space)),
                ),
                drop_item_from_keyboard.run_if(
                    tile_interaction_enabled
                        .and_then(any_with_component::<Player>)
                        .and_then(input_just_pressed(KeyCode::KeyK)),
                ),
                confirm_care_menu_from_keyboard.run_if(
                    in_state(Menu::CareItemPicker)
                        .and_then(any_with_component::<Player>)
                        .and_then(
                            input_just_pressed(KeyCode::Enter)
                                .or_else(input_just_pressed(KeyCode::Space)),
                        ),
                ),
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

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::state::app::StatesPlugin;

    #[derive(Resource, Default)]
    struct CapturedCommands(Vec<GameCommand>);

    fn capture_command(trigger: On<GameCommand>, mut captured: ResMut<CapturedCommands>) {
        captured.0.push(trigger.event().clone());
    }

    fn keyboard_app(screen: Screen, menu: Menu) -> App {
        let mut app = App::new();
        app.add_plugins((StatesPlugin, MinimalPlugins));
        app.add_plugins(alveus_app::plugin);
        app.init_resource::<ButtonInput<KeyCode>>()
            .init_resource::<CapturedCommands>()
            .add_plugins(InputPlugin)
            .add_observer(capture_command);
        app.world_mut().spawn(Player);
        app.world_mut()
            .resource_mut::<NextState<Screen>>()
            .set(screen);
        app.world_mut().resource_mut::<NextState<Menu>>().set(menu);
        app.update();
        app.world_mut().resource_mut::<CapturedCommands>().0.clear();
        app
    }

    fn press_once(screen: Screen, menu: Menu, key: KeyCode) -> Vec<String> {
        let mut app = keyboard_app(screen, menu);
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(key);
        app.update();
        app.world()
            .resource::<CapturedCommands>()
            .0
            .iter()
            .map(command_name)
            .collect()
    }

    fn command_name(command: &GameCommand) -> String {
        match command {
            GameCommand::Move(intent) => format!("Move::{intent:?}"),
            GameCommand::MoveStop => "MoveStop".into(),
            GameCommand::Interact => "Interact".into(),
            GameCommand::DropItem => "DropItem".into(),
            GameCommand::EnterBuilding => "EnterBuilding".into(),
            GameCommand::ExitRoom => "ExitRoom".into(),
            GameCommand::PauseToggle => "PauseToggle".into(),
            GameCommand::Play => "Play".into(),
            GameCommand::Back => "Back".into(),
            GameCommand::SkipSplash => "SkipSplash".into(),
            GameCommand::OpenSettings => "OpenSettings".into(),
            GameCommand::OpenCredits => "OpenCredits".into(),
            GameCommand::Continue => "Continue".into(),
            GameCommand::QuitToTitle => "QuitToTitle".into(),
            GameCommand::ImproveStat { .. } => "ImproveStat".into(),
            GameCommand::WorsenStat { .. } => "WorsenStat".into(),
            GameCommand::AdvanceTime { .. } => "AdvanceTime".into(),
            GameCommand::AdjustVolume { .. } => "AdjustVolume".into(),
            GameCommand::Screenshot { .. } => "Screenshot".into(),
            GameCommand::AdvanceFrames(_) => "AdvanceFrames".into(),
        }
    }

    #[test]
    fn gameplay_keys_emit_the_canonical_verbs() {
        let cases: &[(KeyCode, &[&str])] = &[
            (KeyCode::KeyW, &["Move::Up"]),
            (KeyCode::ArrowUp, &["Move::Up"]),
            (KeyCode::KeyS, &["Move::Down"]),
            (KeyCode::ArrowDown, &["Move::Down"]),
            (KeyCode::KeyA, &["Move::Left"]),
            (KeyCode::ArrowLeft, &["Move::Left"]),
            (KeyCode::KeyD, &["Move::Right"]),
            (KeyCode::ArrowRight, &["Move::Right"]),
            (KeyCode::Space, &["MoveStop", "Interact"]),
            (KeyCode::KeyK, &["MoveStop", "DropItem"]),
            (KeyCode::KeyP, &["MoveStop", "PauseToggle"]),
            (KeyCode::Escape, &["MoveStop", "PauseToggle"]),
        ];

        for (key, expected) in cases {
            assert_eq!(
                press_once(Screen::Gameplay, Menu::None, *key),
                *expected,
                "unexpected mapping for {key:?}"
            );
        }
    }

    #[test]
    fn splash_and_overlay_keys_emit_the_canonical_verbs() {
        assert_eq!(
            press_once(Screen::Splash, Menu::None, KeyCode::Escape),
            ["SkipSplash"]
        );
        assert_eq!(
            press_once(Screen::Gameplay, Menu::Settings, KeyCode::KeyP),
            ["PauseToggle"]
        );
    }

    #[test]
    fn care_picker_keys_emit_navigation_confirm_and_cancel_verbs() {
        let cases = [
            (KeyCode::KeyW, "Move::Up"),
            (KeyCode::ArrowUp, "Move::Up"),
            (KeyCode::KeyS, "Move::Down"),
            (KeyCode::ArrowDown, "Move::Down"),
            (KeyCode::Enter, "Continue"),
            (KeyCode::Space, "Continue"),
            (KeyCode::Escape, "Back"),
        ];

        for (key, expected) in cases {
            assert_eq!(
                press_once(Screen::Gameplay, Menu::CareItemPicker, key),
                [expected],
                "unexpected care-picker mapping for {key:?}"
            );
        }
    }
}
