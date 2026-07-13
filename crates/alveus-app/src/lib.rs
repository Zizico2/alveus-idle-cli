//! App-wide states and the ordered `Update` system-set schedule shared by every
//! gameplay crate. This is the lowest layer that behaviour crates depend on and
//! the single owner that initializes [`Screen`], [`Menu`], and [`Pause`].

use bevy::prelude::*;
use strum::EnumIter;

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
#[derive(Clone, Copy, Eq, PartialEq, Hash, Debug, Default, Reflect, EnumIter)]
pub enum InRoom {
    #[default]
    NutritionHouse,
    PushPopEnclosure,
    /// Reserved for a future pasture interior; not enterable in gameplay yet.
    Pasture,
    /// Reserved for a future reptile enclosure interior; not enterable in gameplay yet.
    ReptileEnclosure,
}

/// The game's main screen states.
#[derive(States, Copy, Clone, Eq, PartialEq, Hash, Debug, Default, Reflect, EnumIter)]
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
#[derive(States, Copy, Clone, Eq, PartialEq, Hash, Debug, Default, Reflect, EnumIter)]
pub enum Menu {
    #[default]
    None,
    Main,
    Credits,
    Settings,
    Pause,
    /// In-world care item picker (fridge, etc.). Cursor via Move Up/Down;
    /// confirm with Interact/Continue; cancel with Back.
    CareItemPicker,
}

/// Returns whether `screen` represents a world where tile interaction exists.
///
/// Overlay menus are deliberately not considered here; use
/// [`tile_interaction_allowed`] when both state axes are available.
pub fn screen_supports_tile_interaction(screen: Screen) -> bool {
    matches!(screen, Screen::Gameplay | Screen::InRoom(_))
}

/// Canonical policy for player movement and interaction with world tiles.
///
/// Tile interaction is available on the overview and in every room interior,
/// but any active overlay menu owns input until it is closed.
pub fn tile_interaction_allowed(screen: Screen, menu: Menu) -> bool {
    screen_supports_tile_interaction(screen) && menu == Menu::None
}

/// Bevy run-condition adapter for [`tile_interaction_allowed`].
pub fn tile_interaction_enabled(screen: Res<State<Screen>>, menu: Res<State<Menu>>) -> bool {
    tile_interaction_allowed(*screen.get(), *menu.get())
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
    use strum::IntoEnumIterator;

    /// Every concrete [`Screen`] value, expanding [`Screen::InRoom`] across
    /// all [`InRoom`] variants (`EnumIter` alone only yields one default room).
    fn all_screens() -> impl Iterator<Item = Screen> {
        Screen::iter()
            .filter(|screen| !matches!(screen, Screen::InRoom(_)))
            .chain(InRoom::iter().map(Screen::InRoom))
    }

    fn expected_tile_interaction_policy(screen: Screen, menu: Menu) -> bool {
        match screen {
            Screen::Gameplay | Screen::InRoom(_) => match menu {
                Menu::None => true,
                Menu::Main
                | Menu::Credits
                | Menu::Settings
                | Menu::Pause
                | Menu::CareItemPicker => false,
            },
            Screen::Splash | Screen::Title | Screen::Loading | Screen::FatalError => match menu {
                Menu::None
                | Menu::Main
                | Menu::Credits
                | Menu::Settings
                | Menu::Pause
                | Menu::CareItemPicker => false,
            },
        }
    }

    #[test]
    fn tile_interaction_policy_covers_every_screen_and_menu_variant() {
        for screen in all_screens() {
            for menu in Menu::iter() {
                assert_eq!(
                    tile_interaction_allowed(screen, menu),
                    expected_tile_interaction_policy(screen, menu),
                    "unexpected tile-interaction policy for {screen:?} + {menu:?}"
                );
            }
        }
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
}
