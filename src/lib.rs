// Support configuring Bevy lints within code.
#![cfg_attr(bevy_lint, feature(register_tool), register_tool(bevy))]
// Disable console on Windows for non-dev builds.
#![cfg_attr(not(feature = "dev"), windows_subsystem = "windows")]

pub mod asset_tracking;
pub mod components;
pub mod audio;
pub mod demo;
#[cfg(feature = "dev")]
pub mod dev_tools;
pub mod menus;
pub mod screens;
pub mod theme;
pub mod stats;
pub mod hud;

use bevy::{asset::AssetMetaCheck, prelude::*};

pub fn run() -> AppExit {
    App::new().add_plugins(AppPlugin).run()
}

pub struct AppPlugin;

impl Plugin for AppPlugin {
    fn build(&self, app: &mut App) {
        // Add Bevy plugins.
        app.add_plugins(
            DefaultPlugins
                .set(AssetPlugin {
                    // Wasm builds will check for meta files (that don't exist) if this isn't set.
                    // This causes errors and even panics on web build on itch.
                    // See https://github.com/bevyengine/bevy_github_ci_template/issues/48.
                    meta_check: AssetMetaCheck::Never,
                    ..default()
                })
                .set(WindowPlugin {
                    primary_window: Window {
                        title: "Alveus Idle Cli".to_string(),
                        fit_canvas_to_parent: true,
                        ..default()
                    }
                    .into(),
                    ..default()
                }),
        );

        app.register_type::<components::BuildingEntrance>()
            .register_type::<components::TileGroup>()
            .register_type::<components::RectangleTileGroup>()
            .register_type::<components::TilePosition>()
            .register_type::<components::Obstacle>();

        app.add_plugins((
            asset_tracking::plugin,
            audio::plugin,
            demo::plugin,
            #[cfg(feature = "dev")]
            dev_tools::plugin,
            menus::plugin,
            screens::plugin,
            theme::plugin,
            bevy_tweening::TweeningPlugin,
            stats::StatsPlugin,
            hud::HudPlugin,
        ));

        // Order new `AppSystems` variants by adding them here:
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

        // Set up the `Pause` state.
        app.init_state::<Pause>();
        app.configure_sets(Update, PausableSystems.run_if(in_state(Pause(false))));

        // Spawn the main camera.
        app.add_systems(Startup, spawn_camera);
    }
}

/// High-level groupings of systems for the app in the `Update` schedule.
/// When adding a new variant, make sure to order it in the `configure_sets`
/// call above.
#[derive(SystemSet, Debug, Clone, Copy, Eq, PartialEq, Hash, PartialOrd, Ord)]
pub enum AppSystems {
    /// Tick timers.
    TickTimers,
    /// Record player input.
    RecordInput,
    /// Systems that run decay logic.
    DecayCalculation,
    /// Systems that update global upkeep.
    UpkeepCalculation,
    /// Systems that update HUD / UI text/bars.
    UiUpdate,
    /// Systems that periodically save/autosave state.
    SaveSystem,
    /// Do everything else.
    Update,
}

/// Whether or not the game is paused.
#[derive(States, Copy, Clone, Eq, PartialEq, Hash, Debug, Default)]
pub struct Pause(pub bool);

/// A system set for systems that shouldn't run while the game is paused.
#[derive(SystemSet, Copy, Clone, Eq, PartialEq, Hash, Debug)]
pub struct PausableSystems;

pub fn spawn_camera(mut commands: Commands) {
    commands.spawn((
        Name::new("Camera"),
        Camera2d,
        bevy::camera_controller::pan_camera::PanCamera {
            key_up: None,
            key_down: None,
            key_left: None,
            key_right: None,
            ..default()
        },
    ));
}
