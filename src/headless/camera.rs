//! Offscreen render target for windowless headless mode.

use bevy::{
    asset::RenderAssetUsages,
    camera::RenderTarget,
    prelude::*,
    render::render_resource::{Extent3d, TextureDimension, TextureFormat, TextureUsages},
};

use crate::demo::camera::camera_follow;

/// Default offscreen resolution (width x height).
pub const DEFAULT_HEADLESS_RESOLUTION: (u32, u32) = (1280, 720);

#[derive(Resource, Debug, Clone, Reflect)]
#[reflect(Resource)]
pub struct HeadlessRenderTarget {
    pub image: Handle<Image>,
    pub width: u32,
    pub height: u32,
}

#[derive(Resource, Debug, Clone, Copy)]
pub struct HeadlessResolution(pub (u32, u32));

pub struct HeadlessCameraPlugin {
    pub resolution: (u32, u32),
}

impl Default for HeadlessCameraPlugin {
    fn default() -> Self {
        Self {
            resolution: DEFAULT_HEADLESS_RESOLUTION,
        }
    }
}

impl Plugin for HeadlessCameraPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(HeadlessResolution(self.resolution))
            .add_systems(Startup, spawn_headless_camera)
            .add_systems(Update, camera_follow);
    }
}

fn spawn_headless_camera(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    resolution: Res<HeadlessResolution>,
) {
    let (width, height) = resolution.0;
    let size = Extent3d {
        width,
        height,
        depth_or_array_layers: 1,
    };

    let mut image = Image::new_fill(
        size,
        TextureDimension::D2,
        &[0, 0, 0, 255],
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    );
    image.texture_descriptor.usage =
        TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_SRC;

    let handle = images.add(image);

    commands.insert_resource(HeadlessRenderTarget {
        image: handle.clone(),
        width,
        height,
    });

    commands.spawn((
        Name::new("Headless Camera"),
        Camera2d,
        Camera::default(),
        RenderTarget::Image(handle.into()),
    ));
}
