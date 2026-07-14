//! Local keyboard/gamepad input contexts mapped to canonical [`GameCommand`]s.
//!
//! BRP clients trigger the same verbs directly. Contexts only decide which
//! physical controls own input; gameplay validity remains enforced by the
//! command and simulation layers.

use alveus_app::{Menu, Screen};
use alveus_command::GameCommand;
use alveus_components::MovementIntent;
use alveus_configs::{DEBUG_ADVANCE_HOURS, DEBUG_STAT_IMPROVE_AMOUNT, DEBUG_STAT_WORSEN_AMOUNT};
use alveus_menus_models::ListMenuDirection;
use alveus_stats::{
    AnimalId, AnimalStat, AnimalStats, EnclosureId, EnclosureStat, EnclosureStats, StatTarget,
};
use bevy::{
    input_focus::{
        InputFocus, InputFocusVisible, directional_navigation::DirectionalNavigationPlugin,
        tab_navigation::TabNavigationPlugin,
    },
    math::CompassOctant,
    prelude::*,
    ui::auto_directional_navigation::AutoDirectionalNavigator,
    ui_widgets::Activate,
};
use bevy_enhanced_input::prelude::*;

/// Registers enhanced local-input contexts and their focus bridge.
pub struct InputPlugin;

impl Plugin for InputPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            EnhancedInputPlugin,
            DirectionalNavigationPlugin,
            TabNavigationPlugin,
        ))
        .add_input_context::<GameplayInput>()
        .add_input_context::<CarePickerInput>()
        .add_input_context::<MenuInput>()
        .add_input_context::<SplashInput>()
        .add_input_context::<DebugInput>()
        .add_systems(Startup, spawn_input_contexts)
        .add_systems(
            PreUpdate,
            sync_input_contexts.before(EnhancedInputSystems::Update),
        )
        .add_observer(move_player)
        .add_observer(stop_player)
        .add_observer(interact)
        .add_observer(drop_item)
        .add_observer(enter_building)
        .add_observer(exit_room)
        .add_observer(toggle_pause)
        .add_observer(navigate_care_picker)
        .add_observer(confirm_care_picker)
        .add_observer(close_care_picker)
        .add_observer(navigate_menu)
        .add_observer(confirm_menu)
        .add_observer(close_menu)
        .add_observer(skip_splash)
        .add_observer(improve_polly_hunger)
        .add_observer(improve_polly_cleanliness)
        .add_observer(improve_polly_happiness)
        .add_observer(improve_stompy_hunger)
        .add_observer(improve_stompy_cleanliness)
        .add_observer(improve_stompy_happiness)
        .add_observer(improve_georgie_hunger)
        .add_observer(improve_georgie_cleanliness)
        .add_observer(improve_georgie_happiness)
        .add_observer(improve_siren_hunger)
        .add_observer(improve_siren_cleanliness)
        .add_observer(improve_siren_happiness)
        .add_observer(improve_push_pop_hunger)
        .add_observer(improve_push_pop_cleanliness)
        .add_observer(improve_push_pop_happiness)
        .add_observer(worsen_all_stats)
        .add_observer(advance_debug_time)
        .add_observer(toggle_debug_ui);
    }
}

#[derive(Component)]
pub struct GameplayInput;

#[derive(Component)]
pub struct CarePickerInput;

#[derive(Component)]
pub struct MenuInput;

#[derive(Component)]
pub struct SplashInput;

#[derive(Component)]
pub struct DebugInput;

/// Local development-only request to toggle Bevy's UI debug overlay.
#[derive(Event, Debug, Clone, Copy)]
pub struct ToggleUiDebug;

#[derive(InputAction)]
#[action_output(Vec2)]
pub struct WorldMove;

#[derive(InputAction)]
#[action_output(bool)]
pub struct Interact;

#[derive(InputAction)]
#[action_output(bool)]
pub struct DropItem;

#[derive(InputAction)]
#[action_output(bool)]
pub struct EnterBuilding;

#[derive(InputAction)]
#[action_output(bool)]
pub struct ExitRoom;

#[derive(InputAction)]
#[action_output(bool)]
pub struct Pause;

#[derive(InputAction)]
#[action_output(f32)]
pub struct CareNavigate;

#[derive(InputAction)]
#[action_output(bool)]
pub struct CareConfirm;

#[derive(InputAction)]
#[action_output(bool)]
pub struct CareBack;

#[derive(InputAction)]
#[action_output(Vec2)]
pub struct MenuNavigate;

#[derive(InputAction)]
#[action_output(bool)]
pub struct MenuConfirm;

#[derive(InputAction)]
#[action_output(bool)]
pub struct MenuBack;

#[derive(InputAction)]
#[action_output(bool)]
pub struct SkipSplash;

macro_rules! debug_actions {
    ($($action:ident),+ $(,)?) => {
        $(
            #[derive(InputAction)]
            #[action_output(bool)]
            pub struct $action;
        )+
    };
}

debug_actions!(
    PollyHunger,
    PollyCleanliness,
    PollyHappiness,
    StompyHunger,
    StompyCleanliness,
    StompyHappiness,
    GeorgieHunger,
    GeorgieCleanliness,
    GeorgieHappiness,
    SirenHunger,
    SirenCleanliness,
    SirenHappiness,
    PushPopHunger,
    PushPopCleanliness,
    PushPopHappiness,
    WorsenAllStats,
    AdvanceDebugTime,
    ToggleDebugUi,
);

fn guarded_action() -> ActionSettings {
    ActionSettings {
        require_reset: true,
        consume_input: true,
        ..default()
    }
}

fn spawn_input_contexts(mut commands: Commands) {
    commands.spawn((
        Name::new("Gameplay Input Context"),
        GameplayInput,
        ContextActivity::<GameplayInput>::INACTIVE,
        actions!(GameplayInput[
            (
                Action::<WorldMove>::new(),
                guarded_action(),
                DeadZone::default(),
                Bindings::spawn((
                    Cardinal::wasd_keys(),
                    Cardinal::arrows(),
                    Cardinal::dpad(),
                    Axial::left_stick(),
                )),
            ),
            (
                Action::<Interact>::new(),
                guarded_action(),
                bindings![KeyCode::Space, GamepadButton::South],
            ),
            (
                Action::<DropItem>::new(),
                guarded_action(),
                bindings![KeyCode::KeyK, GamepadButton::West],
            ),
            (
                Action::<EnterBuilding>::new(),
                guarded_action(),
                bindings![KeyCode::Enter, GamepadButton::North],
            ),
            (
                Action::<ExitRoom>::new(),
                guarded_action(),
                bindings![KeyCode::Backspace, GamepadButton::East],
            ),
            (
                Action::<Pause>::new(),
                guarded_action(),
                bindings![KeyCode::Escape, KeyCode::KeyP, GamepadButton::Start],
            ),
        ]),
    ));

    commands.spawn((
        Name::new("Care Picker Input Context"),
        CarePickerInput,
        ContextActivity::<CarePickerInput>::INACTIVE,
        actions!(CarePickerInput[
            (
                Action::<CareNavigate>::new(),
                guarded_action(),
                DeadZone::default(),
                Bindings::spawn((
                    Bidirectional::new(KeyCode::KeyW, KeyCode::KeyS),
                    Bidirectional::new(KeyCode::ArrowUp, KeyCode::ArrowDown),
                    Bidirectional::new(GamepadButton::DPadUp, GamepadButton::DPadDown),
                    Spawn(Binding::from(GamepadAxis::LeftStickY)),
                )),
            ),
            (
                Action::<CareConfirm>::new(),
                guarded_action(),
                bindings![KeyCode::Enter, KeyCode::Space, GamepadButton::South],
            ),
            (
                Action::<CareBack>::new(),
                guarded_action(),
                bindings![
                    KeyCode::Escape,
                    KeyCode::KeyP,
                    GamepadButton::East,
                    GamepadButton::Start,
                ],
            ),
        ]),
    ));

    commands.spawn((
        Name::new("Menu Input Context"),
        MenuInput,
        ContextActivity::<MenuInput>::INACTIVE,
        actions!(MenuInput[
            (
                Action::<MenuNavigate>::new(),
                guarded_action(),
                DeadZone::default(),
                Bindings::spawn((
                    Cardinal::wasd_keys(),
                    Cardinal::arrows(),
                    Cardinal::dpad(),
                    Axial::left_stick(),
                )),
            ),
            (
                Action::<MenuConfirm>::new(),
                guarded_action(),
                bindings![GamepadButton::South],
            ),
            (
                Action::<MenuBack>::new(),
                guarded_action(),
                bindings![
                    KeyCode::Escape,
                    KeyCode::KeyP,
                    GamepadButton::East,
                    GamepadButton::Start,
                ],
            ),
        ]),
    ));

    commands.spawn((
        Name::new("Splash Input Context"),
        SplashInput,
        ContextActivity::<SplashInput>::INACTIVE,
        actions!(
            SplashInput[(
                Action::<SkipSplash>::new(),
                guarded_action(),
                bindings![
                    KeyCode::Escape,
                    KeyCode::Enter,
                    KeyCode::Space,
                    GamepadButton::South,
                    GamepadButton::Start,
                ],
            )]
        ),
    ));

    commands.spawn((
        Name::new("Debug Input Context"),
        DebugInput,
        ContextActivity::<DebugInput>::INACTIVE,
        actions!(DebugInput[
            (Action::<PollyHunger>::new(), guarded_action(), bindings![KeyCode::Digit1]),
            (Action::<PollyCleanliness>::new(), guarded_action(), bindings![KeyCode::Digit2]),
            (Action::<PollyHappiness>::new(), guarded_action(), bindings![KeyCode::Digit3]),
            (Action::<StompyHunger>::new(), guarded_action(), bindings![KeyCode::Digit4]),
            (Action::<StompyCleanliness>::new(), guarded_action(), bindings![KeyCode::Digit5]),
            (Action::<StompyHappiness>::new(), guarded_action(), bindings![KeyCode::Digit6]),
            (Action::<GeorgieHunger>::new(), guarded_action(), bindings![KeyCode::Digit7]),
            (Action::<GeorgieCleanliness>::new(), guarded_action(), bindings![KeyCode::Digit8]),
            (Action::<GeorgieHappiness>::new(), guarded_action(), bindings![KeyCode::Digit9]),
            (Action::<SirenHunger>::new(), guarded_action(), bindings![KeyCode::Digit0]),
            (Action::<SirenCleanliness>::new(), guarded_action(), bindings![KeyCode::KeyI]),
            (Action::<SirenHappiness>::new(), guarded_action(), bindings![KeyCode::KeyO]),
            (Action::<PushPopHunger>::new(), guarded_action(), bindings![KeyCode::KeyU]),
            (Action::<PushPopCleanliness>::new(), guarded_action(), bindings![KeyCode::KeyJ]),
            (Action::<PushPopHappiness>::new(), guarded_action(), bindings![KeyCode::KeyY]),
            (
                Action::<WorsenAllStats>::new(),
                guarded_action(),
                bindings![KeyCode::Minus, KeyCode::NumpadSubtract, KeyCode::KeyM],
            ),
            (
                Action::<AdvanceDebugTime>::new(),
                guarded_action(),
                bindings![KeyCode::Equal, KeyCode::NumpadAdd, KeyCode::KeyL],
            ),
            (
                Action::<ToggleDebugUi>::new(),
                guarded_action(),
                bindings![KeyCode::F12],
            ),
        ]),
    ));
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum InputLayer {
    None,
    Splash,
    Gameplay,
    CarePicker,
    Menu,
}

fn sync_input_contexts(
    mut commands: Commands,
    screen: Res<State<Screen>>,
    menu: Res<State<Menu>>,
    gameplay: Single<Entity, With<GameplayInput>>,
    care: Single<Entity, With<CarePickerInput>>,
    menu_context: Single<Entity, With<MenuInput>>,
    splash: Single<Entity, With<SplashInput>>,
    debug: Single<Entity, With<DebugInput>>,
    mut focus: ResMut<InputFocus>,
    mut last_layer: Local<Option<InputLayer>>,
) {
    let layer = if *menu.get() == Menu::CareItemPicker {
        InputLayer::CarePicker
    } else if *menu.get() != Menu::None {
        InputLayer::Menu
    } else {
        match screen.get() {
            Screen::Splash => InputLayer::Splash,
            Screen::Gameplay | Screen::InRoom(_) => InputLayer::Gameplay,
            Screen::Title | Screen::Loading | Screen::FatalError => InputLayer::None,
        }
    };
    if *last_layer == Some(layer) {
        return;
    }

    commands
        .entity(*gameplay)
        .insert(ContextActivity::<GameplayInput>::new(
            layer == InputLayer::Gameplay,
        ));
    commands
        .entity(*care)
        .insert(ContextActivity::<CarePickerInput>::new(
            layer == InputLayer::CarePicker,
        ));
    commands
        .entity(*menu_context)
        .insert(ContextActivity::<MenuInput>::new(layer == InputLayer::Menu));
    commands
        .entity(*splash)
        .insert(ContextActivity::<SplashInput>::new(
            layer == InputLayer::Splash,
        ));
    commands
        .entity(*debug)
        .insert(ContextActivity::<DebugInput>::new(
            layer == InputLayer::Gameplay,
        ));
    if matches!(
        layer,
        InputLayer::None | InputLayer::Splash | InputLayer::Gameplay
    ) {
        focus.clear();
    }
    *last_layer = Some(layer);
}

fn movement_intent(value: Vec2) -> Option<MovementIntent> {
    if value.length_squared() < 0.01 {
        return None;
    }
    if value.y.abs() >= value.x.abs() {
        Some(if value.y > 0.0 {
            MovementIntent::Up
        } else {
            MovementIntent::Down
        })
    } else {
        Some(if value.x > 0.0 {
            MovementIntent::Right
        } else {
            MovementIntent::Left
        })
    }
}

fn move_player(event: On<Fire<WorldMove>>, mut commands: Commands) {
    if let Some(intent) = movement_intent(event.value) {
        commands.trigger(GameCommand::Move(intent));
    }
}

fn stop_player(_: On<Complete<WorldMove>>, mut commands: Commands) {
    commands.trigger(GameCommand::MoveStop);
}

// Given an observer name, action type, and game command, defines a function that
// triggers the given command when the specified input action starts.
//
// Example usage:
//     command_observer!(interact, Interact, GameCommand::Interact);
// Expands to:
//     fn interact(_: On<Start<Interact>>, mut commands: Commands) {
//         commands.trigger(GameCommand::Interact);
//     }
macro_rules! command_observer {
    ($name:ident, $action:ty, $command:expr) => {
        fn $name(_: On<Start<$action>>, mut commands: Commands) {
            commands.trigger($command);
        }
    };
}

command_observer!(interact, Interact, GameCommand::Interact);
command_observer!(drop_item, DropItem, GameCommand::DropItem);
command_observer!(enter_building, EnterBuilding, GameCommand::EnterBuilding);
command_observer!(exit_room, ExitRoom, GameCommand::ExitRoom);
command_observer!(toggle_pause, Pause, GameCommand::PauseToggle);
command_observer!(confirm_care_picker, CareConfirm, GameCommand::Continue);
command_observer!(close_care_picker, CareBack, GameCommand::Back);
command_observer!(close_menu, MenuBack, GameCommand::Back);
command_observer!(skip_splash, SkipSplash, GameCommand::SkipSplash);

fn navigate_care_picker(event: On<Start<CareNavigate>>, mut commands: Commands) {
    let direction = if event.value > 0.0 {
        ListMenuDirection::Up
    } else {
        ListMenuDirection::Down
    };
    commands.trigger(GameCommand::NavigateListMenu(direction));
}

fn navigate_menu(
    event: On<Start<MenuNavigate>>,
    menu: Res<State<Menu>>,
    mut commands: Commands,
    mut navigator: AutoDirectionalNavigator,
    mut focus_visible: ResMut<InputFocusVisible>,
) {
    match *menu.get() {
        Menu::Main | Menu::Pause => {
            let Some(intent) = movement_intent(event.value) else {
                return;
            };
            let direction = match intent {
                MovementIntent::Up => ListMenuDirection::Up,
                MovementIntent::Down => ListMenuDirection::Down,
                MovementIntent::Left | MovementIntent::Right => return,
            };
            commands.trigger(GameCommand::NavigateListMenu(direction));
        }
        Menu::Settings | Menu::Credits => {
            let Some(intent) = movement_intent(event.value) else {
                return;
            };
            let direction = match intent {
                MovementIntent::Up => CompassOctant::North,
                MovementIntent::Down => CompassOctant::South,
                MovementIntent::Left => CompassOctant::West,
                MovementIntent::Right => CompassOctant::East,
            };
            focus_visible.0 = true;
            let _ = navigator.navigate(direction);
        }
        Menu::None | Menu::CareItemPicker => {}
    }
}

fn confirm_menu(_: On<Start<MenuConfirm>>, focus: Res<InputFocus>, mut commands: Commands) {
    if let Some(entity) = focus.get() {
        commands.trigger(Activate { entity });
    }
}

macro_rules! improve_stat_observer {
    ($function:ident, $action:ty, $animal:expr, $stat:expr) => {
        fn $function(_: On<Start<$action>>, mut commands: Commands) {
            commands.trigger(GameCommand::ImproveStat {
                target: StatTarget::Animal {
                    id: $animal,
                    stat: $stat,
                },
                amount: DEBUG_STAT_IMPROVE_AMOUNT,
            });
        }
    };
}

improve_stat_observer!(
    improve_polly_hunger,
    PollyHunger,
    AnimalId::Polly,
    AnimalStat::Hunger
);
improve_stat_observer!(
    improve_polly_cleanliness,
    PollyCleanliness,
    AnimalId::Polly,
    AnimalStat::Cleanliness
);
improve_stat_observer!(
    improve_polly_happiness,
    PollyHappiness,
    AnimalId::Polly,
    AnimalStat::Happiness
);
improve_stat_observer!(
    improve_stompy_hunger,
    StompyHunger,
    AnimalId::Stompy,
    AnimalStat::Hunger
);
improve_stat_observer!(
    improve_stompy_cleanliness,
    StompyCleanliness,
    AnimalId::Stompy,
    AnimalStat::Cleanliness
);
improve_stat_observer!(
    improve_stompy_happiness,
    StompyHappiness,
    AnimalId::Stompy,
    AnimalStat::Happiness
);
improve_stat_observer!(
    improve_georgie_hunger,
    GeorgieHunger,
    AnimalId::Georgie,
    AnimalStat::Hunger
);
improve_stat_observer!(
    improve_georgie_cleanliness,
    GeorgieCleanliness,
    AnimalId::Georgie,
    AnimalStat::Cleanliness
);
improve_stat_observer!(
    improve_georgie_happiness,
    GeorgieHappiness,
    AnimalId::Georgie,
    AnimalStat::Happiness
);
improve_stat_observer!(
    improve_siren_hunger,
    SirenHunger,
    AnimalId::Siren,
    AnimalStat::Hunger
);
improve_stat_observer!(
    improve_siren_cleanliness,
    SirenCleanliness,
    AnimalId::Siren,
    AnimalStat::Cleanliness
);
improve_stat_observer!(
    improve_siren_happiness,
    SirenHappiness,
    AnimalId::Siren,
    AnimalStat::Happiness
);
improve_stat_observer!(
    improve_push_pop_hunger,
    PushPopHunger,
    AnimalId::PushPop,
    AnimalStat::Hunger
);
improve_stat_observer!(
    improve_push_pop_cleanliness,
    PushPopCleanliness,
    AnimalId::PushPop,
    AnimalStat::Cleanliness
);
improve_stat_observer!(
    improve_push_pop_happiness,
    PushPopHappiness,
    AnimalId::PushPop,
    AnimalStat::Happiness
);

fn worsen_all_stats(
    _: On<Start<WorsenAllStats>>,
    animals: Query<&AnimalId, With<AnimalStats>>,
    enclosures: Query<&EnclosureId, With<EnclosureStats>>,
    mut commands: Commands,
) {
    for id in &animals {
        for stat in [AnimalStat::Hunger, AnimalStat::Happiness] {
            commands.trigger(GameCommand::WorsenStat {
                target: StatTarget::Animal { id: *id, stat },
                amount: DEBUG_STAT_WORSEN_AMOUNT,
            });
        }
    }
    for id in &enclosures {
        commands.trigger(GameCommand::WorsenStat {
            target: StatTarget::Enclosure {
                id: *id,
                stat: EnclosureStat::Cleanliness,
            },
            amount: DEBUG_STAT_WORSEN_AMOUNT,
        });
    }
}

fn advance_debug_time(_: On<Start<AdvanceDebugTime>>, mut commands: Commands) {
    commands.trigger(GameCommand::AdvanceTime {
        hours: DEBUG_ADVANCE_HOURS,
    });
}

fn toggle_debug_ui(_: On<Start<ToggleDebugUi>>, mut commands: Commands) {
    commands.trigger(ToggleUiDebug);
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::input::InputPlugin as BevyInputPlugin;
    use bevy::input_focus::InputFocusPlugin;
    use bevy::state::app::StatesPlugin;

    #[derive(Resource, Default)]
    struct Captured(Vec<GameCommand>);

    fn capture(event: On<GameCommand>, mut captured: ResMut<Captured>) {
        captured.0.push(event.event().clone());
    }

    fn input_app(screen: Screen, menu: Menu) -> App {
        let mut app = App::new();
        app.add_plugins((
            MinimalPlugins,
            BevyInputPlugin,
            InputFocusPlugin,
            StatesPlugin,
        ));
        app.add_plugins(alveus_app::plugin)
            .init_resource::<Captured>()
            .add_plugins(InputPlugin)
            .add_observer(capture);
        app.world_mut()
            .resource_mut::<NextState<Screen>>()
            .set(screen);
        app.world_mut().resource_mut::<NextState<Menu>>().set(menu);
        app.finish();
        app.cleanup();
        app.update();
        app.world_mut().resource_mut::<Captured>().0.clear();
        app
    }

    #[test]
    fn care_navigation_mock_emits_one_semantic_move() {
        let mut app = input_app(Screen::Gameplay, Menu::CareItemPicker);
        let context = app
            .world_mut()
            .query_filtered::<Entity, With<CarePickerInput>>()
            .single(app.world())
            .unwrap();
        app.world_mut()
            .entity_mut(context)
            .mock_once::<CarePickerInput, CareNavigate>(TriggerState::Fired, 1.0)
            .unwrap();
        app.update();
        assert!(matches!(
            app.world().resource::<Captured>().0.as_slice(),
            [GameCommand::NavigateListMenu(ListMenuDirection::Up)]
        ));
    }

    #[test]
    fn care_keyboard_navigation_fires_once_until_released() {
        let mut app = input_app(Screen::Gameplay, Menu::CareItemPicker);
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::ArrowDown);
        app.update();
        app.update();
        assert!(matches!(
            app.world().resource::<Captured>().0.as_slice(),
            [GameCommand::NavigateListMenu(ListMenuDirection::Down)]
        ));
    }

    #[test]
    fn held_back_does_not_bleed_into_new_context() {
        let mut app = input_app(Screen::Gameplay, Menu::None);
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::Escape);
        app.update();
        assert!(matches!(
            app.world().resource::<Captured>().0.as_slice(),
            [GameCommand::PauseToggle]
        ));

        app.world_mut().resource_mut::<Captured>().0.clear();
        app.world_mut()
            .resource_mut::<NextState<Menu>>()
            .set(Menu::CareItemPicker);
        app.update();
        assert!(app.world().resource::<Captured>().0.is_empty());

        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .release(KeyCode::Escape);
        app.update();
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::Escape);
        app.update();
        assert!(matches!(
            app.world().resource::<Captured>().0.as_slice(),
            [GameCommand::Back]
        ));
    }

    #[test]
    fn gameplay_and_care_contexts_are_exclusive() {
        let mut app = input_app(Screen::Gameplay, Menu::None);
        let gameplay = **app
            .world_mut()
            .query_filtered::<&ContextActivity<GameplayInput>, With<GameplayInput>>()
            .single(app.world())
            .unwrap();
        let care = **app
            .world_mut()
            .query_filtered::<&ContextActivity<CarePickerInput>, With<CarePickerInput>>()
            .single(app.world())
            .unwrap();
        assert!(gameplay);
        assert!(!care);
    }
}
