//! Assert that Push Pop's feeding dish hydrates `FeedAnimal` with a typed
//! `FeedStat` from the migrated Tiled class property shape.
//!
//! Self-contained (does not `mod common`) so unused shared helpers do not
//! produce dead-code warnings under clippy `-D warnings`.

use std::path::{Path, PathBuf};

use alveus_configs::CARE_FEED_RESTORE;
use alveus_content::ItemId;
use alveus_interaction::FeedAnimal;
use alveus_types::{AnimalId, FeedStat, Stat};
use bevy::asset::AssetMetaCheck;
use bevy::asset::io::memory::{Dir, MemoryAssetReader};
use bevy::asset::io::{AssetSourceBuilder, AssetSourceId};
use bevy::image::{CompressedImageFormats, ImageLoader};
use bevy::prelude::*;
use bevy::render::{ExtractSchedule, RenderApp};
use bevy::time::TimePlugin;
use bevy_ecs_tiled::prelude::*;

fn seed_maps_assets(dir: &Dir) {
    let assets_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets");
    copy_dir_into_memory(dir, &assets_root, Path::new(""));
}

fn copy_dir_into_memory(dir: &Dir, disk_root: &Path, rel: &Path) {
    for entry in std::fs::read_dir(disk_root.join(rel))
        .unwrap_or_else(|e| panic!("failed to read {}: {e}", disk_root.join(rel).display()))
    {
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

fn tiled_spawn_app() -> App {
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
        TransformPlugin,
    ));
    app.init_asset::<Image>();
    app.register_asset_loader(ImageLoader::new(CompressedImageFormats::empty()));

    // `bevy_ecs_tilemap` with the `render` feature expects a `RenderApp` sub-app.
    let mut render_app = SubApp::new();
    render_app.init_schedule(ExtractSchedule);
    app.insert_sub_app(RenderApp, render_app);

    app.register_type::<alveus_components::Obstacle>()
        .register_type::<alveus_content::RoomObjectId>()
        .register_type::<alveus_types::ItemId>()
        .register_type::<alveus_components::Interactable>()
        .register_type::<FeedAnimal>()
        .register_type::<AnimalId>()
        .register_type::<Stat>()
        .register_type::<FeedStat>();

    app.add_plugins(TiledPlugin(TiledPluginConfig {
        tiled_types_export_file: None,
        tiled_types_filter: TiledFilter::All,
    }));

    app
}

fn wait_for_map_spawn(app: &mut App, handle: &Handle<TiledMapAsset>) {
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
        if !state.is_loaded()
            || app
                .world()
                .resource::<Assets<TiledMapAsset>>()
                .get(handle)
                .is_none()
        {
            continue;
        }
        // Properties are inserted during map spawn after the asset is ready.
        if app
            .world_mut()
            .query::<&FeedAnimal>()
            .iter(app.world())
            .next()
            .is_some()
        {
            return;
        }
    }
    panic!(
        "timeout waiting for FeedAnimal on spawned push_pop interior (property deserialization likely failed)"
    );
}

#[test]
fn push_pop_interior_hydrates_feed_animal_stat() {
    let mut app = tiled_spawn_app();

    let handle = {
        let assets = app.world().resource::<AssetServer>();
        assets.load("maps/interiors/push_pop_enclosure_interior.tmx")
    };
    app.world_mut().spawn(TiledMap(handle.clone()));
    wait_for_map_spawn(&mut app, &handle);

    let feeds: Vec<FeedAnimal> = app
        .world_mut()
        .query::<&FeedAnimal>()
        .iter(app.world())
        .cloned()
        .collect();

    assert_eq!(
        feeds.len(),
        1,
        "expected exactly one FeedAnimal from the feeding dish tile, got {feeds:?}"
    );
    let feed = &feeds[0];
    assert_eq!(feed.animal_id, AnimalId::PushPop);
    assert_eq!(feed.required_item, ItemId::TortoiseLeafyGreens);
    assert_eq!(feed.delta, FeedStat(Stat(1000)));
    assert_eq!(feed.delta, CARE_FEED_RESTORE);
    assert_eq!(feed.prompt, "Place leafy greens for Push Pop");
}
