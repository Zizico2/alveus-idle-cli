// Support configuring Bevy lints within code.
#![cfg_attr(bevy_lint, feature(register_tool), register_tool(bevy))]
// Disable console on Windows for non-dev builds.
#![cfg_attr(not(feature = "dev"), windows_subsystem = "windows")]

//! Composition root for the Alveus idle game. This crate wires the feature
//! crates together into an [`App`]; almost all game logic lives in the
//! `crates/alveus-*` workspace members.

#[cfg(feature = "dev")]
mod dev_tools;

use std::thread;
use std::time::Duration;

use bevy::{asset::AssetMetaCheck, prelude::*};

#[cfg(feature = "headless")]
use alveus_headless::HeadlessPlugin;
use alveus_headless::{CommandPlugin, DEFAULT_HEADLESS_RESOLUTION, InputPlugin, StepRequest};

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
            port: alveus_headless::DEFAULT_BRP_PORT,
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

    let default_plugins = DefaultPlugins
        .set(AssetPlugin {
            meta_check: AssetMetaCheck::Never,
            ..default()
        })
        .set(window_plugin);
    if headless {
        // Headless mode has no primary window and must not create a Winit event
        // loop, which requires a desktop compositor even when no window exists.
        app.add_plugins(default_plugins.disable::<bevy::winit::WinitPlugin>());
        app.add_plugins(bevy::app::ScheduleRunnerPlugin::default());
    } else {
        app.add_plugins(default_plugins);
    }

    // Single canonical Reflect registration (shared with the headless server and
    // the `gen_tiled_types` exporter).
    alveus_headless::register_headless_types(&mut app);

    app.add_plugins((
        alveus_app::plugin,
        alveus_asset_tracking::plugin,
        alveus_audio::plugin,
        // Collision before world so required map handles are requested before
        // `load_resource::<LevelAssets/InteriorAssets>` wraps them as dependencies.
        alveus_collision::CollisionPlugin,
        alveus_world::plugin,
        alveus_menus::plugin,
        alveus_screens::plugin,
        alveus_theme::plugin,
        alveus_stats::StatsPlugin,
        alveus_interaction::InteractionPlugin,
        alveus_cleaning::CleaningPlugin,
        alveus_animals::AnimalsPlugin,
        alveus_hud::HudPlugin,
        CommandPlugin,
        InputPlugin,
    ));

    #[cfg(feature = "dev")]
    app.add_plugins(dev_tools::plugin);

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
        app.add_systems(Startup, spawn_camera);
    }

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

pub fn spawn_camera(mut commands: Commands) {
    commands.spawn((
        Name::new("Camera"),
        Camera2d,
        IsDefaultUiCamera,
        bevy::camera_controller::pan_camera::PanCamera {
            key_up: None,
            key_down: None,
            key_left: None,
            key_right: None,
            ..default()
        },
    ));
}
