//! App-wide states and the ordered `Update` system-set schedule shared by every
//! gameplay crate. This is the lowest layer that behaviour crates depend on and
//! the single owner that initializes [`Screen`], [`Menu`], and [`Pause`].

use bevy::prelude::*;

/// Adds a unique plugin unless an earlier feature plugin already registered it.
///
/// This lets independently composed feature plugins declare a shared internal
/// dependency without requiring their parent composition root to know about it.
pub fn ensure_plugin<P: Plugin>(app: &mut App, plugin: P) {
    if !app.is_plugin_added::<P>() {
        app.add_plugins(plugin);
    }
}

/// High-level groupings of systems for the app in the `Update` schedule.
#[derive(SystemSet, Debug, Clone, Copy, Eq, PartialEq, Hash, PartialOrd, Ord)]
pub enum AppSystems {
    TickTimers,
    RecordInput,
    DecayCalculation,
    UpkeepCalculation,
    UiUpdate,
    SaveSystem,
    Update,
}

/// Whether or not the game is paused.
#[derive(States, Copy, Clone, Eq, PartialEq, Hash, Debug, Default, Reflect)]
pub struct Pause(pub bool);

/// A system set for systems that shouldn't run while the game is paused.
#[derive(SystemSet, Copy, Clone, Eq, PartialEq, Hash, Debug)]
pub struct PausableSystems;

/// A room interior the player can be inside.
#[derive(Clone, Copy, Eq, PartialEq, Hash, Debug, Reflect)]
pub enum InRoom {
    NutritionHouse,
    PushPopEnclosure,
    /// Reserved for a future pasture interior; not enterable in gameplay yet.
    Pasture,
    /// Reserved for a future reptile enclosure interior; not enterable in gameplay yet.
    ReptileEnclosure,
}

/// The game's main screen states.
#[derive(States, Copy, Clone, Eq, PartialEq, Hash, Debug, Default, Reflect)]
pub enum Screen {
    #[default]
    Splash,
    Title,
    Loading,
    /// A required startup asset failed or Loading timed out. The current process
    /// cannot safely enter gameplay; the player must restart the game.
    FatalError,
    Gameplay,
    InRoom(InRoom),
}

/// The game's menu states.
#[derive(States, Copy, Clone, Eq, PartialEq, Hash, Debug, Default, Reflect)]
pub enum Menu {
    #[default]
    None,
    Main,
    Credits,
    Settings,
    Pause,
    /// Care item picker overlay (fridge, etc.). Cursor via Move Up/Down;
    /// confirm with Interact/Continue; cancel with Back.
    CareItemPicker,
}

/// True while world movement and tile interaction may run.
///
/// Menus own input while open. Keeping this predicate in the app-state crate
/// lets keyboard, interaction, player movement, and animal movement share the
/// same definition instead of drifting apart.
pub fn tile_interaction_enabled(screen: Res<State<Screen>>, menu: Res<State<Menu>>) -> bool {
    tile_interaction_enabled_for(*screen.get(), *menu.get())
}

/// Value-based form of [`tile_interaction_enabled`] for exclusive-world command
/// dispatch, which cannot use ordinary system parameters.
pub fn tile_interaction_enabled_for(screen: Screen, menu: Menu) -> bool {
    matches!(screen, Screen::Gameplay | Screen::InRoom(_)) && menu == Menu::None
}

/// Initializes all app-wide states and configures the shared `Update` system-set
/// ordering.
///
/// Add this plugin before feature plugins that consume [`Screen`], [`Menu`], or
/// [`Pause`].
pub fn plugin(app: &mut App) {
    app.init_state::<Screen>();
    app.init_state::<Menu>();
    app.init_state::<Pause>();

    app.configure_sets(
        Update,
        (
            AppSystems::TickTimers,
            AppSystems::RecordInput,
            AppSystems::DecayCalculation,
            AppSystems::UpkeepCalculation,
            AppSystems::UiUpdate,
            AppSystems::SaveSystem,
            AppSystems::Update,
        )
            .chain(),
    );

    app.configure_sets(Update, PausableSystems.run_if(in_state(Pause(false))));
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::state::app::StatesPlugin;

    #[derive(Resource, Default)]
    struct PluginBuildCount(usize);

    struct SharedPlugin;

    impl Plugin for SharedPlugin {
        fn build(&self, app: &mut App) {
            app.init_resource::<PluginBuildCount>();
            app.world_mut().resource_mut::<PluginBuildCount>().0 += 1;
        }
    }

    struct FirstConsumer;

    impl Plugin for FirstConsumer {
        fn build(&self, app: &mut App) {
            ensure_plugin(app, SharedPlugin);
        }
    }

    struct SecondConsumer;

    impl Plugin for SecondConsumer {
        fn build(&self, app: &mut App) {
            ensure_plugin(app, SharedPlugin);
        }
    }

    #[test]
    fn ensure_plugin_registers_a_shared_dependency_once() {
        let mut app = App::new();

        app.add_plugins((FirstConsumer, SecondConsumer));

        assert_eq!(app.world().resource::<PluginBuildCount>().0, 1);
    }

    #[test]
    fn plugin_owns_app_wide_state_defaults_and_transitions() {
        let mut app = App::new();
        app.add_plugins(StatesPlugin);
        app.add_plugins(plugin);

        assert_eq!(
            *app.world().resource::<State<Screen>>().get(),
            Screen::Splash
        );
        assert_eq!(*app.world().resource::<State<Menu>>().get(), Menu::None);
        assert_eq!(*app.world().resource::<State<Pause>>().get(), Pause(false));
        assert!(app.world().contains_resource::<NextState<Screen>>());
        assert!(app.world().contains_resource::<NextState<Menu>>());
        assert!(app.world().contains_resource::<NextState<Pause>>());

        app.world_mut()
            .resource_mut::<NextState<Menu>>()
            .set(Menu::Settings);
        app.update();

        assert_eq!(*app.world().resource::<State<Menu>>().get(), Menu::Settings);
    }

    #[test]
    fn tile_interaction_requires_a_playable_screen_without_an_overlay() {
        assert!(tile_interaction_enabled_for(Screen::Gameplay, Menu::None));
        assert!(tile_interaction_enabled_for(
            Screen::InRoom(InRoom::NutritionHouse),
            Menu::None
        ));
        assert!(!tile_interaction_enabled_for(
            Screen::Gameplay,
            Menu::CareItemPicker
        ));
        assert!(!tile_interaction_enabled_for(Screen::Title, Menu::None));
    }
}
