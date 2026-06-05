//! Player-specific behavior.

use bevy::{
    image::{ImageLoaderSettings, ImageSampler},
    prelude::*,
};

use crate::components::{CurrentTilePosition, DesiredTilePosition, TilePosition};
use crate::demo::level::TILE_SIZE;

use crate::{
    AppSystems, PausableSystems,
    asset_tracking::LoadResource,
    demo::{
        animation::PlayerAnimation,
        movement::{
            MovementController,
            MovementDuration,
            MovementIntent, // DISABLED
                            //  ScreenWrap
        },
    },
};

pub(super) fn plugin(app: &mut App) {
    app.load_resource::<PlayerAssets>();

    // Record directional input as movement controls.
    app.add_systems(
        Update,
        record_player_directional_input
            .in_set(AppSystems::RecordInput)
            .in_set(PausableSystems),
    );
}

pub const PLAYER_Z_INDEX: f32 = 2.0;

/// The player character.
pub fn player(
    max_speed: f32,
    player_assets: &PlayerAssets,
    texture_atlas_layouts: &mut Assets<TextureAtlasLayout>,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<ColorMaterial>,
    spawn_pos: TilePosition,
) -> impl Bundle {
    // A texture atlas is a way to split a single image into a grid of related images.
    // You can learn more in this example: https://github.com/bevyengine/bevy/blob/latest/examples/2d/texture_atlas.rs
    let layout = TextureAtlasLayout::from_grid(UVec2::splat(32), 6, 2, Some(UVec2::splat(1)), None);
    let texture_atlas_layout = texture_atlas_layouts.add(layout);
    let player_animation = PlayerAnimation::new();

    (
        Name::new("Player"),
        Player,
        // Sprite::from_atlas_image(
        //     player_assets.ducky.clone(),
        //     TextureAtlas {
        //         layout: texture_atlas_layout,
        //         index: player_animation.get_atlas_index(),
        //     },
        // ),
        // Transform::from_scale(Vec2::splat(8.0).extend(1.0)),
        Mesh2d(meshes.add(Circle::new(16.))),
        MeshMaterial2d(materials.add(Color::srgb(0.3, 0.1, 0.9))),
        Transform::from_xyz(
            spawn_pos.x as f32 * TILE_SIZE as f32,
            spawn_pos.y as f32 * TILE_SIZE as f32,
            PLAYER_Z_INDEX,
        ),
        MovementController {
            // max_speed,
            ..default()
        },
        // DISABLED
        // ScreenWrap,
        player_animation,
    )
}

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Default, Reflect)]
#[reflect(Component)]
#[require(
    CurrentTilePosition,
    DesiredTilePosition,
    MovementDuration(Timer::from_seconds(0.25, TimerMode::Once))
)]
pub struct Player;



impl PartialEq<CurrentTilePosition> for DesiredTilePosition {
    fn eq(&self, other: &CurrentTilePosition) -> bool {
        self.0 == other.0
    }
}
impl PartialEq<DesiredTilePosition> for CurrentTilePosition {
    fn eq(&self, other: &DesiredTilePosition) -> bool {
        self.0 == other.0
    }
}

// #[derive(Clone, Copy, Debug, Default, Component)]
// pub struct MovementLock;

fn record_player_directional_input(
    input: Res<ButtonInput<KeyCode>>,
    mut controller_query: Single<&mut MovementController, With<Player>>,
) {
    // Collect directional input.

    let intent = if input.pressed(KeyCode::KeyW) || input.pressed(KeyCode::ArrowUp) {
        Some(MovementIntent::Up)
    } else if input.pressed(KeyCode::KeyS) || input.pressed(KeyCode::ArrowDown) {
        Some(MovementIntent::Down)
    } else if input.pressed(KeyCode::KeyA) || input.pressed(KeyCode::ArrowLeft) {
        Some(MovementIntent::Left)
    } else if input.pressed(KeyCode::KeyD) || input.pressed(KeyCode::ArrowRight) {
        Some(MovementIntent::Right)
    } else {
        None
    };

    // Apply movement intent to controller.
    controller_query.intent = intent;
}

#[derive(Resource, Asset, Clone, Reflect)]
#[reflect(Resource)]
pub struct PlayerAssets {
    #[dependency]
    ducky: Handle<Image>,
    #[dependency]
    pub steps: Vec<Handle<AudioSource>>,
}

impl FromWorld for PlayerAssets {
    fn from_world(world: &mut World) -> Self {
        let assets = world.resource::<AssetServer>();
        Self {
            ducky: assets.load_with_settings(
                "images/ducky.png",
                |settings: &mut ImageLoaderSettings| {
                    // Use `nearest` image sampling to preserve pixel art style.
                    settings.sampler = ImageSampler::nearest();
                },
            ),
            steps: vec![
                assets.load("audio/sound_effects/step1.ogg"),
                assets.load("audio/sound_effects/step2.ogg"),
                assets.load("audio/sound_effects/step3.ogg"),
                assets.load("audio/sound_effects/step4.ogg"),
            ],
        }
    }
}
