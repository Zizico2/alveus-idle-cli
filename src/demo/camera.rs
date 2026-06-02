use bevy::prelude::*;
use bevy::camera_controller::pan_camera::PanCameraPlugin;
use crate::demo::player::Player;

pub struct CameraControllerPlugin;

impl Plugin for CameraControllerPlugin {
    fn build(&self, app: &mut App) {
        // Register the built-in PanCameraPlugin
        app.add_plugins(PanCameraPlugin);
        
        // Add the custom camera follow system
        app.add_systems(Update, camera_follow);
    }
}

pub const CAMERA_FOLLOW_SPEED: f32 = 4.0;

fn camera_follow(
    time: Res<Time>,
    player_transform: Single<&Transform, With<Player>>,
    mut camera_query: Query<&mut Transform, (With<Camera2d>, Without<Player>)>,
) {
    for mut camera_transform in &mut camera_query {
        let target = player_transform.translation.xy();
        let current = camera_transform.translation.xy();
        
        let decay = 1.0 - f32::exp(-CAMERA_FOLLOW_SPEED * time.delta_secs());
        let new_pos = current.lerp(target, decay);
        
        camera_transform.translation.x = new_pos.x;
        camera_transform.translation.y = new_pos.y;
    }
}
