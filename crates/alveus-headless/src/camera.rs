//! Offscreen render target for windowless headless mode.

use bevy::{
    asset::RenderAssetUsages,
    camera::RenderTarget,
    prelude::*,
    render::render_resource::{Extent3d, TextureDimension, TextureFormat, TextureUsages},
};

pub use alveus_command::HeadlessRenderTarget;
use alveus_world::camera::camera_follow;

/// Default offscreen resolution (width x height).
pub const DEFAULT_HEADLESS_RESOLUTION: (u32, u32) = (1280, 720);

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
        // Windowless mode has no primary window, so UI roots without
        // `UiTargetCamera` only reach this offscreen target when it is marked
        // as the default UI camera (world + HUD/menus/toasts in screenshots).
        IsDefaultUiCamera,
    ));
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::camera::RenderTarget;

    fn headless_camera_app(resolution: (u32, u32)) -> App {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default()))
            .init_asset::<Image>()
            .add_plugins(HeadlessCameraPlugin { resolution });
        app.update();
        app
    }

    #[test]
    fn headless_camera_is_default_ui_camera_with_image_target() {
        let mut app = headless_camera_app((320, 180));

        let mut query = app
            .world_mut()
            .query::<(&Camera, Option<&IsDefaultUiCamera>)>();
        let cameras: Vec<_> = query.iter(app.world()).collect();
        assert_eq!(cameras.len(), 1, "expected exactly one camera");
        assert!(
            cameras[0].1.is_some(),
            "headless Camera2d must carry IsDefaultUiCamera"
        );

        let expected = app.world().resource::<HeadlessRenderTarget>().image.clone();
        let target = app
            .world_mut()
            .query_filtered::<&RenderTarget, With<Camera>>()
            .single(app.world())
            .expect("camera RenderTarget");
        match target {
            RenderTarget::Image(image_target) => {
                assert_eq!(image_target.handle, expected);
            }
            other => panic!("expected RenderTarget::Image, got {other:?}"),
        }
    }

    #[test]
    fn exactly_one_default_ui_camera() {
        let mut app = headless_camera_app(DEFAULT_HEADLESS_RESOLUTION);
        let count = app
            .world_mut()
            .query_filtered::<Entity, (With<Camera>, With<IsDefaultUiCamera>)>()
            .iter(app.world())
            .count();
        assert_eq!(count, 1);
    }

    #[test]
    fn render_target_matches_configured_resolution() {
        let resolution = (640, 360);
        let app = headless_camera_app(resolution);
        let target = app.world().resource::<HeadlessRenderTarget>();
        assert_eq!((target.width, target.height), resolution);

        let handle = target.image.clone();
        let image = app
            .world()
            .resource::<Assets<Image>>()
            .get(&handle)
            .expect("render target image");
        assert_eq!(image.width(), resolution.0);
        assert_eq!(image.height(), resolution.1);
    }
}
