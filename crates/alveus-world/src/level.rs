//! Spawn the main level and configure the Tiled integration.

use std::path::{Path, PathBuf};

use bevy::prelude::*;
use bevy_ecs_tiled::prelude::{regex::RegexSet, *};
use bevy_ecs_tiled::tiled::properties::export_types;

use alveus_asset_tracking::LoadResource;
use alveus_collision::{InteriorAssets, LevelAssets};
use alveus_components::{CurrentTilePosition, DesiredTilePosition, TILE_SIZE};

use crate::player::player;
use crate::room::PlayerSpawnPoint;

/// Filter for Reflect types exported into `tiled_types.json`.
///
/// Shared by [`tiled_plugin`] (gameplay) and [`export_tiled_types`] (standalone
/// `gen_tiled_types` binary).
pub fn tiled_types_filter() -> TiledFilter {
    TiledFilter::from(
        RegexSet::new([
            r"^alveus_components::.*",
            r"^alveus_content::.*",
            r"^alveus_interaction::.*",
            r"^alveus_cleaning::.*",
            r"^alveus_stats::.*",
            r"^alveus_types::.*",
        ])
        .unwrap(),
    )
}

/// Builds the [`TiledPlugin`] with the game's user-property type filter.
///
/// Pass `Some(path)` only when you want the plugin's Startup system to rewrite
/// the export file; normal gameplay should pass `None`. Prefer
/// [`export_tiled_types`] for the standalone exporter — it avoids pulling in
/// `TilemapPlugin` / `RenderApp`.
pub fn tiled_plugin(export_to: Option<PathBuf>) -> TiledPlugin {
    TiledPlugin(TiledPluginConfig {
        tiled_types_export_file: export_to,
        tiled_types_filter: tiled_types_filter(),
    })
}

/// Write `tiled_types.json` from the app's type registry without adding
/// [`TiledPlugin`] (which requires a render sub-app when `render` is enabled).
pub fn export_tiled_types(app: &App, path: impl AsRef<Path>) {
    let reg = app.world().resource::<AppTypeRegistry>();
    export_types(reg, path, &tiled_types_filter());
}

pub(super) fn plugin(app: &mut App) {
    app.add_plugins(tiled_plugin(None))
        .load_resource::<LevelAssets>()
        .load_resource::<InteriorAssets>();
}

/// A system that spawns the main level.
pub fn spawn_level(
    mut commands: Commands,
    level_assets: Res<LevelAssets>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    spawn_point: Res<PlayerSpawnPoint>,
) {
    let spawn_pos = spawn_point.position;
    commands.spawn((
        Name::new("Level"),
        Transform::default(),
        Visibility::default(),
        DespawnOnExit(alveus_app::Screen::Gameplay),
        children![
            (
                player(&mut meshes, &mut materials, spawn_pos),
                CurrentTilePosition(spawn_pos),
                DesiredTilePosition(spawn_pos),
            ),
            (
                Name::new("Overview Map"),
                TiledMap(level_assets.map.clone()),
                TilemapAnchor::BottomLeft,
                Transform::from_xyz(-(TILE_SIZE as f32 / 2.0), -(TILE_SIZE as f32 / 2.0), 0.0,),
            )
        ],
    ));
}
