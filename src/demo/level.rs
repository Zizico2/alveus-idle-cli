//! Spawn the main level.

use std::{env, path::PathBuf};

use bevy::prelude::*;
use bevy_ecs_tiled::prelude::{regex::RegexSet, *};

use crate::{
    asset_tracking::LoadResource,
    audio::music,
    demo::player::{PlayerAssets, player},
    screens::Screen,
};

pub const TILE_SIZE: u32 = 32;

pub(super) fn plugin(app: &mut App) {
    let tiled_types_path = env::current_dir()
        .unwrap()
        .join("assets")
        .join("maps")
        .join("overview")
        .join("tiled_types.json");

    app.add_plugins(TiledPlugin(TiledPluginConfig {
        tiled_types_export_file: Some(tiled_types_path),
        // Filter out internal Bevy components to keep the Tiled export clean
        // tiled_types_filter: TiledFilter::Names(vec![
        //     "alveus_idle::components::BuildingEntrance".into(),
        // ]),
        tiled_types_filter: TiledFilter::from(
            RegexSet::new([r"^alveus_idle_cli::components::.*"]).unwrap(),
        ),
    }))
    // .init_asset::<TiledMapAsset>()
    .load_resource::<LevelAssets>();
}

#[derive(Resource, Asset, Clone, Reflect)]
#[reflect(Resource)]
pub struct LevelAssets {
    // #[dependency]
    // music: Handle<AudioSource>,
    #[dependency]
    enter_building_toast: Handle<Image>,
    #[dependency]
    map: Handle<TiledMapAsset>,
}

impl FromWorld for LevelAssets {
    fn from_world(world: &mut World) -> Self {
        let assets = world.resource::<AssetServer>();
        Self {
            // music: assets.load("audio/music/Fluffing A Duck.ogg"),
            enter_building_toast: assets.load("images/enter_building_toast.png"),
            map: assets.load("maps/overview/map.tmx"),
        }
    }
}

/// A system that spawns the main level.
pub fn spawn_level(
    mut commands: Commands,
    level_assets: Res<LevelAssets>,
    player_assets: Res<PlayerAssets>,
    mut texture_atlas_layouts: ResMut<Assets<TextureAtlasLayout>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    commands.spawn((
        Name::new("Level"),
        Transform::default(),
        Visibility::default(),
        DespawnOnExit(Screen::Gameplay),
        children![
            player(400.0, &player_assets, &mut texture_atlas_layouts, &mut meshes, &mut materials),
            // (
            //     Name::new("Gameplay Music"),
            //     music(level_assets.music.clone())
            // )
            (
                Name::new("Overview Map"), 
                TiledMap(level_assets.map.clone()),
                TilemapAnchor::BottomLeft,
            )
        ],
    ));
}
