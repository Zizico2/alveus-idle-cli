//! Windowless rendering and BRP transports.

mod camera;
#[cfg(feature = "cli")]
mod stdio;

pub use camera::{
    DEFAULT_HEADLESS_RESOLUTION, HeadlessCameraPlugin, HeadlessRenderTarget, HeadlessResolution,
};
pub const DEFAULT_BRP_PORT: u16 = 15702;

use bevy::prelude::*;
#[cfg(feature = "remote")]
use bevy::remote::{RemotePlugin, http::RemoteHttpPlugin};

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
        alveus_reflect::register_agent_types(app);

        app.add_plugins(HeadlessCameraPlugin {
            resolution: self.resolution,
        });

        #[cfg(feature = "remote")]
        {
            app.add_plugins(RemotePlugin::default())
                .add_plugins(RemoteHttpPlugin::default().with_port(self.http_port));
        }

        #[cfg(feature = "cli")]
        if self.enable_stdio {
            app.add_plugins(stdio::StdioBrpPlugin);
        }
    }
}
