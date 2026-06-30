use std::path::{Path, PathBuf};

use alveus_idle_cli::headless::CommandPlugin;
use alveus_idle_cli::screens::Screen;
use alveus_idle_cli::stats::{SavePath, StatsPlugin};
use bevy::asset::io::memory::{Dir, MemoryAssetReader};
use bevy::asset::io::{AssetSourceBuilder, AssetSourceId};
use bevy::asset::AssetMetaCheck;
use bevy::image::{CompressedImageFormats, ImageLoader};
use bevy::prelude::*;
use bevy::state::app::StatesPlugin;
use bevy::time::TimePlugin;
use bevy_ecs_tiled::prelude::*;

pub fn minimal_stats_app(save_path: &str) -> App {
    let mut app = App::new();
    app.add_plugins(StatesPlugin);
    app.init_state::<Screen>();
    app.add_plugins(MinimalPlugins);
    app.add_plugins(AssetPlugin::default());
    app.init_resource::<ButtonInput<KeyCode>>();
    app.insert_resource(SavePath(save_path.to_string()));
    app.add_plugins((StatsPlugin, CommandPlugin));
    app.insert_resource(NextState::Pending(Screen::Gameplay));
    app.update();
    app
}

fn seed_maps_assets(dir: &Dir) {
    let assets_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets");
    copy_dir_into_memory(dir, &assets_root, Path::new(""));
}

fn copy_dir_into_memory(dir: &Dir, disk_root: &Path, rel: &Path) {
    for entry in std::fs::read_dir(disk_root.join(rel)).unwrap_or_else(|e| {
        panic!("failed to read {}: {e}", disk_root.join(rel).display())
    }) {
        let entry = entry.expect("valid directory entry");
        let file_name = entry.file_name();
        let child_rel = rel.join(file_name);
        let path = entry.path();
        if path.is_dir() {
            copy_dir_into_memory(dir, disk_root, &child_rel);
        } else {
            let bytes = std::fs::read(&path).expect("map asset bytes");
            dir.insert_asset(&child_rel, bytes);
        }
    }
}

/// Headless app with Tiled asset loading (memory-backed assets, no GPU render stack).
pub fn headless_tiled_test_app() -> App {
    let dir = Dir::default();
    seed_maps_assets(&dir);
    let dir_for_reader = dir.clone();

    let mut app = App::new();
    app.register_asset_source(
        AssetSourceId::Default,
        AssetSourceBuilder::new(move || {
            Box::new(MemoryAssetReader {
                root: dir_for_reader.clone(),
            })
        }),
    );
    app.add_plugins((
        TaskPoolPlugin::default(),
        TimePlugin,
        AssetPlugin {
            meta_check: AssetMetaCheck::Never,
            watch_for_changes_override: Some(false),
            use_asset_processor_override: Some(false),
            ..default()
        },
    ));
    app.init_asset::<Image>();
    app.register_asset_loader(ImageLoader::new(CompressedImageFormats::empty()));

    app.register_type::<alveus_idle_cli::components::BuildingEntrance>()
        .register_type::<alveus_idle_cli::components::TilePosition>()
        .register_type::<alveus_idle_cli::components::Obstacle>()
        .register_type::<alveus_idle_cli::components::InEnclosure>()
        .register_type::<alveus_idle_cli::content::RoomObjectId>()
        .register_type::<alveus_idle_cli::content::ItemId>()
        .register_type::<alveus_idle_cli::interaction::Interactable>()
        .register_type::<alveus_idle_cli::interaction::GiveItem>()
        .register_type::<alveus_idle_cli::interaction::FeedAnimal>()
        .register_type::<alveus_idle_cli::stats::AnimalId>()
        .register_type::<alveus_idle_cli::stats::AnimalStat>();

    app.add_plugins(TiledPlugin(TiledPluginConfig {
        tiled_types_export_file: None,
        tiled_types_filter: TiledFilter::All,
    }));

    app
}

pub fn load_tiled_map(app: &mut App, path: &'static str) -> Handle<TiledMapAsset> {
    let handle = {
        let assets = app.world().resource::<AssetServer>();
        assets.load(path)
    };
    wait_for_tiled_map(app, &handle);
    handle
}

fn wait_for_tiled_map(app: &mut App, handle: &Handle<TiledMapAsset>) {
    use bevy::asset::RecursiveDependencyLoadState;

    for i in 0..10_000 {
        app.update();
        let server = app.world().resource::<AssetServer>();
        let Some(state) = server.get_recursive_dependency_load_state(handle) else {
            continue;
        };
        if matches!(state, RecursiveDependencyLoadState::Failed(_)) {
            panic!("failed to load TiledMapAsset {handle:?} at iter {i}: {state:?}");
        }
        if state.is_loaded()
            && app
                .world()
                .resource::<Assets<TiledMapAsset>>()
                .get(handle)
                .is_some()
        {
            return;
        }
    }
    let server = app.world().resource::<AssetServer>();
    let state = server.get_recursive_dependency_load_state(handle);
    panic!("timeout loading TiledMapAsset {handle:?}, last state: {state:?}");
}

pub fn cleanup_save(save_path: &str) {
    let _ = std::fs::remove_file(save_path);
}
