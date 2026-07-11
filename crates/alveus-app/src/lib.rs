//! App-wide states and the ordered `Update` system-set schedule shared by every
//! gameplay crate. This is the lowest layer that behaviour crates depend on.

use bevy::prelude::*;

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
    /// In-world care item picker (fridge, etc.). Cursor via Move Up/Down;
    /// confirm with Interact/Continue; cancel with Back.
    CareItemPicker,
}

/// Configures the shared `Update` system-set ordering and the [`Pause`] state.
pub fn plugin(app: &mut App) {
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

    app.init_state::<Pause>();
    app.configure_sets(Update, PausableSystems.run_if(in_state(Pause(false))));
}
