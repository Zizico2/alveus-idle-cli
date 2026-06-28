//! Headless / remote control: semantic verbs, offscreen rendering, and BRP transports.

mod camera;
mod command;
pub mod reflect;
pub use reflect::register_headless_types;
#[cfg(feature = "cli")]
mod stdio;

pub use camera::{HeadlessCameraPlugin, HeadlessRenderTarget, DEFAULT_HEADLESS_RESOLUTION};
pub use command::{GameCommand, StepRequest, CommandPlugin};
pub const DEFAULT_BRP_PORT: u16 = 15702;

use bevy::prelude::*;
#[cfg(feature = "remote")]
use bevy::remote::{http::RemoteHttpPlugin, RemotePlugin};

pub struct HeadlessPlugin {
    pub http_port: u16,
    pub resolution: (u32, u32),
    pub enable_stdio: bool,
}

impl Default for HeadlessPlugin {
    fn default() -> Self {
        Self {
            http_port: DEFAULT_BRP_PORT,
            resolution: DEFAULT_HEADLESS_RESOLUTION,
            enable_stdio: cfg!(feature = "cli"),
        }
    }
}

impl Plugin for HeadlessPlugin {
    fn build(&self, app: &mut App) {
        reflect::register_headless_types(app);

        app.add_plugins(HeadlessCameraPlugin {
            resolution: self.resolution,
        });

        #[cfg(feature = "remote")]
        {
            app.add_plugins(RemotePlugin::default()).add_plugins(
                RemoteHttpPlugin::default().with_port(self.http_port),
            );
        }

        #[cfg(feature = "cli")]
        if self.enable_stdio {
            app.add_plugins(stdio::StdioBrpPlugin);
        }
    }
}
