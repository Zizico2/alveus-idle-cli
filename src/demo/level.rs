//! Spawn the main level.

use std::env;

use bevy::prelude::*;
use bevy_ecs_tiled::prelude::{regex::RegexSet, *};

use crate::{
    asset_tracking::LoadResource,
    components::{CurrentTilePosition, DesiredTilePosition},
    demo::player::{PlayerAssets, player},
    demo::room::PlayerSpawnPoint,
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
            RegexSet::new([
                r"^alveus_idle_cli::components::.*",
                r"^alveus_idle_cli::content::.*",
                r"^alveus_idle_cli::interaction::.*",
                r"^alveus_idle_cli::stats::.*",
            ])
            .unwrap(),
        ),
    }))
    // .init_asset::<TiledMapAsset>()
    .load_resource::<LevelAssets>()
    .load_resource::<InteriorAssets>();
}

#[derive(Resource, Asset, Clone, Reflect)]
#[reflect(Resource)]
pub struct LevelAssets {
    // #[dependency]
    // music: Handle<AudioSource>,
    #[dependency]
    pub enter_building_toast: Handle<Image>,
    #[dependency]
    pub map: Handle<TiledMapAsset>,
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

#[derive(Resource, Asset, Clone, Reflect)]
#[reflect(Resource)]
pub struct InteriorAssets {
    #[dependency]
    pub nutrition_house: Handle<TiledMapAsset>,
    #[dependency]
    pub push_pop_enclosure: Handle<TiledMapAsset>,
}

impl FromWorld for InteriorAssets {
    fn from_world(world: &mut World) -> Self {
        let assets = world.resource::<AssetServer>();
        Self {
            nutrition_house: assets.load("maps/interiors/nutrition_house_interior.tmx"),
            push_pop_enclosure: assets.load("maps/interiors/push_pop_enclosure_interior.tmx"),
        }
    }
}

impl InteriorAssets {
    pub fn collision_entries(&self) -> [(crate::stats::EnclosureId, Handle<TiledMapAsset>); 2] {
        use crate::stats::EnclosureId;
        [
            (EnclosureId::NutritionHousePlaypen, self.nutrition_house.clone()),
            (EnclosureId::PushPopEnclosure, self.push_pop_enclosure.clone()),
        ]
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
    spawn_point: Res<PlayerSpawnPoint>,
) {
    let spawn_pos = spawn_point.position;
    commands.spawn((
        Name::new("Level"),
        Transform::default(),
        Visibility::default(),
        DespawnOnExit(Screen::Gameplay),
        children![
            (
                player(400.0, &player_assets, &mut texture_atlas_layouts, &mut meshes, &mut materials, spawn_pos),
                CurrentTilePosition(spawn_pos),
                DesiredTilePosition(spawn_pos),
            ),
            // (
            //     Name::new("Gameplay Music"),
            //     music(level_assets.music.clone())
            // )
            (
                Name::new("Overview Map"),
                TiledMap(level_assets.map.clone()),
                TilemapAnchor::BottomLeft,
                Transform::from_xyz(
                    -(TILE_SIZE as f32 / 2.0),
                    -(TILE_SIZE as f32 / 2.0),
                    0.0,
                ),
            )
        ],
    ));
}
