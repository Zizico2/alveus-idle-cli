use std::path::{Path, PathBuf};

use alveus_app::Screen;
use alveus_headless::CommandPlugin;
use alveus_stats::{SavePath, StatsPlugin};
use bevy::asset::AssetMetaCheck;
use bevy::asset::io::memory::{Dir, MemoryAssetReader};
use bevy::asset::io::{AssetSourceBuilder, AssetSourceId};
use bevy::image::{CompressedImageFormats, ImageLoader};
use bevy::prelude::*;
use bevy::render::{ExtractSchedule, RenderApp};
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

    // `bevy_ecs_tilemap` with the `render` feature expects a `RenderApp` sub-app
    // (for array-texture preload extract). Asset-only tests don't need a GPU
    // backend — a stub sub-app with `ExtractSchedule` is enough.
    let mut render_app = SubApp::new();
    render_app.init_schedule(ExtractSchedule);
    app.insert_sub_app(RenderApp, render_app);

    app.register_type::<alveus_components::BuildingEntrance>()
        .register_type::<alveus_types::TilePosition>()
        .register_type::<alveus_components::Obstacle>()
        .register_type::<alveus_components::InEnclosure>()
        .register_type::<alveus_content::RoomObjectId>()
        .register_type::<alveus_types::ItemId>()
        .register_type::<alveus_components::Interactable>()
        .register_type::<alveus_interaction::GiveItem>()
        .register_type::<alveus_interaction::FeedAnimal>()
        .register_type::<alveus_interaction::EnrichAnimal>()
        .register_type::<alveus_interaction::CleanAnimal>()
        .register_type::<alveus_interaction::MiniChore>()
        .register_type::<alveus_interaction::OpenMenu>()
        .register_type::<alveus_types::AnimalId>()
        .register_type::<alveus_types::ChoreId>()
        .register_type::<alveus_types::CareMenuId>()
        .register_type::<alveus_types::Stat>()
        .register_type::<alveus_types::FeedStat>()
        .register_type::<alveus_types::EnrichStat>()
        .register_type::<alveus_types::CleanStat>()
        .register_type::<alveus_stats::AnimalStat>();

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

/// Headless app that exercises Title → Loading → Gameplay with real map assets.
///
/// Loads `LevelAssets` + `InteriorAssets` through [`ResourceHandles`] (same path as
/// the game) and builds collision masks while in Loading.
pub fn loading_transition_app() -> App {
    use alveus_asset_tracking::LoadResource;
    use alveus_collision::{
        CollisionMasks, InteriorAssets, LevelAssets, build_all_collision_masks, collision_ready,
    };
    use alveus_menus::PlayClickEvent;
    use bevy_ecs_tiled::prelude::TiledMapAsset;

    let mut app = headless_tiled_test_app();
    app.add_plugins(StatesPlugin);
    app.init_state::<Screen>();
    app.add_plugins(alveus_asset_tracking::plugin);
    app.init_resource::<CollisionMasks>();
    app.load_resource::<LevelAssets>()
        .load_resource::<InteriorAssets>();
    app.add_observer(alveus_menus::main_menu::handle_play_click);

    app.add_systems(
        Update,
        (
            move |mut masks: ResMut<CollisionMasks>,
                  map_assets: Res<Assets<TiledMapAsset>>,
                  level_assets: Option<Res<LevelAssets>>,
                  interior_assets: Option<Res<InteriorAssets>>,
                  screen: Res<State<Screen>>| {
                if *screen.get() != Screen::Loading {
                    return;
                }
                let (Some(level_assets), Some(interior_assets)) = (level_assets, interior_assets)
                else {
                    return;
                };
                if collision_ready(&masks) {
                    return;
                }
                build_all_collision_masks(&mut masks, &map_assets, &level_assets, &interior_assets);
            },
            move |resource_handles: Res<alveus_asset_tracking::ResourceHandles>,
                  masks: Res<CollisionMasks>,
                  screen: Res<State<Screen>>,
                  mut next_screen: ResMut<NextState<Screen>>| {
                if *screen.get() != Screen::Loading {
                    return;
                }
                if resource_handles.is_all_done() && collision_ready(&masks) {
                    next_screen.set(Screen::Gameplay);
                }
            },
        ),
    );

    // Start on Title so Play routes through Loading when assets are still pending.
    app.insert_resource(NextState::Pending(Screen::Title));
    app.update();
    let _ = PlayClickEvent;
    app
}
