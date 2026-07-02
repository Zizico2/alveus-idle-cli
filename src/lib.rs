// Support configuring Bevy lints within code.
#![cfg_attr(bevy_lint, feature(register_tool), register_tool(bevy))]
// Disable console on Windows for non-dev builds.
#![cfg_attr(not(feature = "dev"), windows_subsystem = "windows")]

pub mod animals;
pub mod asset_tracking;
pub mod audio;
pub mod cleaning;
pub mod collision;
pub mod components;
pub mod content;
pub mod demo;
#[cfg(feature = "dev")]
pub mod dev_tools;
pub mod headless;
pub mod hud;
pub mod interaction;
pub mod menus;
pub mod screens;
pub mod stats;
pub mod theme;

use std::thread;
use std::time::Duration;

use bevy::{asset::AssetMetaCheck, prelude::*};

#[cfg(feature = "headless")]
use headless::HeadlessPlugin;
use headless::{CommandPlugin, DEFAULT_HEADLESS_RESOLUTION, StepRequest};

/// How the application is run.
#[derive(Debug, Clone)]
pub enum RunMode {
    Windowed,
    Headless(HeadlessConfig),
}

/// Configuration for windowless headless play with BRP transports.
#[derive(Debug, Clone)]
pub struct HeadlessConfig {
    pub port: u16,
    pub resolution: (u32, u32),
    pub step_mode: bool,
    pub enable_stdio: bool,
}

impl Default for HeadlessConfig {
    fn default() -> Self {
        Self {
            port: headless::DEFAULT_BRP_PORT,
            resolution: DEFAULT_HEADLESS_RESOLUTION,
            step_mode: false,
            enable_stdio: cfg!(feature = "cli"),
        }
    }
}

pub fn run() -> AppExit {
    run_with_args(std::env::args().skip(1).collect())
}

pub fn run_with_args(args: Vec<String>) -> AppExit {
    run_with_mode(parse_run_mode(args))
}

pub fn run_with_mode(mode: RunMode) -> AppExit {
    let step_mode = match &mode {
        RunMode::Headless(cfg) => cfg.step_mode,
        RunMode::Windowed => false,
    };
    let mut app = build_app(mode);
    if step_mode {
        run_headless_loop(&mut app)
    } else {
        app.run()
    }
}

fn parse_run_mode(args: Vec<String>) -> RunMode {
    let mut headless = false;
    let mut port = HeadlessConfig::default().port;
    let mut resolution = DEFAULT_HEADLESS_RESOLUTION;
    let mut step_mode = false;
    let mut enable_stdio = cfg!(feature = "cli");
    let mut no_stdio = false;

    let mut iter = args.into_iter();
    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--headless" => headless = true,
            "--step" => step_mode = true,
            "--realtime" => step_mode = false,
            "--no-stdio" => no_stdio = true,
            "--port" => {
                if let Some(value) = iter.next() {
                    port = value.parse().unwrap_or(port);
                }
            }
            "--resolution" => {
                if let Some(value) = iter.next()
                    && let Some((w, h)) = value.split_once('x')
                    && let (Ok(w), Ok(h)) = (w.parse(), h.parse())
                {
                    resolution = (w, h);
                }
            }
            _ => {}
        }
    }

    if no_stdio {
        enable_stdio = false;
    }

    if headless {
        RunMode::Headless(HeadlessConfig {
            port,
            resolution,
            step_mode,
            enable_stdio,
        })
    } else {
        RunMode::Windowed
    }
}

pub fn build_app(mode: RunMode) -> App {
    let mut app = App::new();
    let headless = matches!(mode, RunMode::Headless(_));

    let window_plugin = match mode {
        RunMode::Windowed => WindowPlugin {
            primary_window: Window {
                title: "Alveus Idle Cli".to_string(),
                fit_canvas_to_parent: true,
                resolution: bevy::window::WindowResolution::default()
                    .with_scale_factor_override(1.0),
                ..default()
            }
            .into(),
            ..default()
        },
        RunMode::Headless(_) => WindowPlugin {
            primary_window: None,
            exit_condition: bevy::window::ExitCondition::DontExit,
            ..default()
        },
    };

    app.add_plugins(
        DefaultPlugins
            .set(AssetPlugin {
                meta_check: AssetMetaCheck::Never,
                ..default()
            })
            .set(window_plugin),
    );

    app.register_type::<components::BuildingEntrance>()
        .register_type::<components::TileGroup>()
        .register_type::<components::RectangleTileGroup>()
        .register_type::<components::TilePosition>()
        .register_type::<components::Obstacle>()
        .register_type::<components::DynamicObstacle>()
        .register_type::<components::InEnclosure>()
        .register_type::<components::PersistedDynamicObstacle>()
        .register_type::<content::RoomObjectId>()
        .register_type::<content::ItemId>()
        .register_type::<interaction::Interactable>()
        .register_type::<interaction::GiveItem>()
        .register_type::<interaction::FeedAnimal>()
        .register_type::<cleaning::PoopPile>()
        .register_type::<cleaning::PoopDump>();

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
        collision::CollisionPlugin,
        interaction::InteractionPlugin,
        cleaning::CleaningPlugin,
        animals::AnimalsPlugin,
        hud::HudPlugin,
        CommandPlugin,
    ));

    if headless {
        #[cfg(feature = "headless")]
        {
            let HeadlessConfig {
                port,
                resolution,
                step_mode: _,
                enable_stdio,
            } = match mode {
                RunMode::Headless(cfg) => cfg,
                RunMode::Windowed => unreachable!(),
            };

            app.add_plugins(HeadlessPlugin {
                http_port: port,
                resolution,
                enable_stdio,
            });
        }
        #[cfg(not(feature = "headless"))]
        {
            panic!("Rebuild with `--features headless` to use --headless mode");
        }
    } else {
        headless::reflect::register_headless_types(&mut app);
        app.add_systems(Startup, spawn_camera);
    }

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

    app
}

fn run_headless_loop(app: &mut App) -> AppExit {
    loop {
        while app.world().resource::<StepRequest>().pending == 0 {
            if app.should_exit().is_some() {
                return AppExit::Success;
            }
            thread::sleep(Duration::from_millis(10));
        }

        let frames = app.world_mut().resource_mut::<StepRequest>().take_all();
        for _ in 0..frames {
            app.update();
            if let Some(exit) = app.should_exit() {
                return exit;
            }
        }
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
