#![allow(dead_code)]

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
    seed_maps_assets_excluding(dir, &[]);
}

fn seed_maps_assets_excluding(dir: &Dir, exclude: &[&str]) {
    let assets_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets");
    copy_dir_into_memory(dir, &assets_root, Path::new(""), exclude);
}

fn copy_dir_into_memory(dir: &Dir, disk_root: &Path, rel: &Path, exclude: &[&str]) {
    for entry in std::fs::read_dir(disk_root.join(rel))
        .unwrap_or_else(|e| panic!("failed to read {}: {e}", disk_root.join(rel).display()))
    {
        let entry = entry.expect("valid directory entry");
        let file_name = entry.file_name();
        let child_rel = rel.join(&file_name);
        let path = entry.path();
        if exclude.iter().any(|e| child_rel == Path::new(e)) {
            continue;
        }
        if path.is_dir() {
            copy_dir_into_memory(dir, disk_root, &child_rel, exclude);
        } else {
            let bytes = std::fs::read(&path).expect("map asset bytes");
            dir.insert_asset(&child_rel, bytes);
        }
    }
}

/// Shared memory asset root for headless tiled tests (so fixtures can be repaired).
#[derive(Resource, Clone)]
pub struct MemoryAssetStore(pub Dir);

/// Like [`headless_tiled_test_app`], but replace the given asset-relative paths with
/// `replacement` bytes after seeding (e.g. corrupt a `.tmx` so the loader fails).
pub fn headless_tiled_test_app_with_replacements(replacements: &[(&str, &[u8])]) -> App {
    let dir = Dir::default();
    seed_maps_assets_excluding(&dir, &[]);
    for (path, bytes) in replacements {
        dir.insert_asset(Path::new(path), bytes.to_vec());
    }
    finish_headless_tiled_app(dir)
}

/// Like [`headless_tiled_test_app`], but omit the given asset-relative paths from the
/// memory store (e.g. `"maps/overview/map.tmx"`) so loads fail with NotFound.
pub fn headless_tiled_test_app_excluding(exclude: &[&str]) -> App {
    let dir = Dir::default();
    seed_maps_assets_excluding(&dir, exclude);
    finish_headless_tiled_app(dir)
}

fn finish_headless_tiled_app(dir: Dir) -> App {
    let dir_for_reader = dir.clone();

    let mut app = App::new();
    app.insert_resource(MemoryAssetStore(dir.clone()));
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

/// Headless app with Tiled asset loading (memory-backed assets, no GPU render stack).
pub fn headless_tiled_test_app() -> App {
    headless_tiled_test_app_excluding(&[])
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

/// When present, the loading diagnostic harness never builds masks / enters
/// Gameplay, so the timeout watchdog can be exercised with fast-loading assets.
#[derive(Resource, Debug, Default, Clone, Copy)]
pub struct StallLoadingForTimeoutTest;

fn test_clear_loading_diagnostics(
    mut failures: ResMut<alveus_collision::CollisionLoadFailures>,
    mut timeout: ResMut<alveus_screens::LoadingTimeoutDiagnostic>,
    mut gate: ResMut<alveus_collision::CollisionReloadGate>,
    mut failed_messages: ResMut<
        Messages<bevy::asset::AssetLoadFailedEvent<bevy_ecs_tiled::prelude::TiledMapAsset>>,
    >,
    asset_server: Res<AssetServer>,
    required: Res<alveus_collision::RequiredCollisionMapHandles>,
    level_assets: Option<Res<alveus_collision::LevelAssets>>,
    interior_assets: Option<Res<alveus_collision::InteriorAssets>>,
) {
    failed_messages.clear();
    let handles = alveus_collision::required_collision_handles(
        &required,
        level_assets.as_deref(),
        interior_assets.as_deref(),
    );
    alveus_collision::reload_failed_collision_maps(&asset_server, &handles, &mut gate);
    failures.clear();
    timeout.clear();
}

fn test_all_required_maps_terminal(
    asset_server: Res<AssetServer>,
    required: Res<alveus_collision::RequiredCollisionMapHandles>,
    level_assets: Option<Res<alveus_collision::LevelAssets>>,
    interior_assets: Option<Res<alveus_collision::InteriorAssets>>,
) -> bool {
    use bevy::asset::RecursiveDependencyLoadState;

    let handles = alveus_collision::required_collision_handles(
        &required,
        level_assets.as_deref(),
        interior_assets.as_deref(),
    );
    handles.iter().all(|(_, handle)| {
        matches!(
            asset_server.get_recursive_dependency_load_state(handle),
            Some(RecursiveDependencyLoadState::Loaded | RecursiveDependencyLoadState::Failed(_))
        )
    })
}

fn test_build_masks_during_loading(
    mut masks: ResMut<alveus_collision::CollisionMasks>,
    map_assets: Res<Assets<bevy_ecs_tiled::prelude::TiledMapAsset>>,
    level_assets: Option<Res<alveus_collision::LevelAssets>>,
    interior_assets: Option<Res<alveus_collision::InteriorAssets>>,
    stall: Option<Res<StallLoadingForTimeoutTest>>,
) {
    use alveus_collision::{build_all_collision_masks, collision_ready};

    if stall.is_some() {
        return;
    }

    if let (Some(level_assets), Some(interior_assets)) = (level_assets, interior_assets)
        && !collision_ready(&masks)
    {
        build_all_collision_masks(&mut masks, &map_assets, &level_assets, &interior_assets);
    }
}

fn test_advance_collision_reload_gate(
    asset_server: Res<AssetServer>,
    required: Res<alveus_collision::RequiredCollisionMapHandles>,
    level_assets: Option<Res<alveus_collision::LevelAssets>>,
    interior_assets: Option<Res<alveus_collision::InteriorAssets>>,
    mut gate: ResMut<alveus_collision::CollisionReloadGate>,
    mut failed_events: MessageReader<
        bevy::asset::AssetLoadFailedEvent<bevy_ecs_tiled::prelude::TiledMapAsset>,
    >,
) {
    let handles = alveus_collision::required_collision_handles(
        &required,
        level_assets.as_deref(),
        interior_assets.as_deref(),
    );
    alveus_collision::advance_collision_reload_gate(
        &asset_server,
        &handles,
        &mut gate,
        failed_events.read(),
    );
}

fn test_detect_collision_failures(
    asset_server: Res<AssetServer>,
    required: Res<alveus_collision::RequiredCollisionMapHandles>,
    level_assets: Option<Res<alveus_collision::LevelAssets>>,
    interior_assets: Option<Res<alveus_collision::InteriorAssets>>,
    mut failures: ResMut<alveus_collision::CollisionLoadFailures>,
    gate: Res<alveus_collision::CollisionReloadGate>,
) {
    let handles = alveus_collision::required_collision_handles(
        &required,
        level_assets.as_deref(),
        interior_assets.as_deref(),
    );
    alveus_collision::record_failed_collision_map_loads(
        &asset_server,
        &handles,
        &mut failures,
        &gate,
    );
}

/// Headless app that exercises Title → Loading → Gameplay with real map assets.
///
/// Loads `LevelAssets` + `InteriorAssets` through [`ResourceHandles`] (same path as
/// the game) and builds collision masks while in Loading.
pub fn loading_transition_app() -> App {
    loading_diagnostic_app(&[])
}

/// Loading harness with optional replaced map bytes and production-like failure /
/// timeout diagnostics (no UI spawn).
pub fn loading_diagnostic_app(map_replacements: &[(&str, &[u8])]) -> App {
    use std::time::Instant;

    use alveus_asset_tracking::LoadResource;
    use alveus_collision::{
        CollisionLoadFailures, CollisionMasks, InteriorAssets, LevelAssets, REQUIRED_COLLISION_KEYS,
        collision_ready,
    };
    use alveus_menus::PlayClickEvent;
    use alveus_screens::{LoadingTimeoutDiagnostic, LoadingTiming};

    #[derive(Resource, Debug)]
    struct TestLoadingWatchdog {
        started: Instant,
    }

    let mut app = headless_tiled_test_app_with_replacements(map_replacements);
    app.add_plugins(StatesPlugin);
    app.init_state::<Screen>();
    app.add_plugins(alveus_asset_tracking::plugin);
    app.init_resource::<CollisionMasks>();
    app.init_resource::<CollisionLoadFailures>();
    app.init_resource::<alveus_collision::CollisionReloadGate>();
    app.init_resource::<LoadingTimeoutDiagnostic>();
    app.init_resource::<LoadingTiming>();
    {
        let handles = alveus_collision::RequiredCollisionMapHandles::from_asset_server(
            app.world().resource::<AssetServer>(),
        );
        app.insert_resource(handles);
    }
    app.load_resource::<LevelAssets>()
        .load_resource::<InteriorAssets>();
    app.add_observer(alveus_menus::main_menu::handle_play_click);

    app.add_systems(
        OnEnter(Screen::Loading),
        (
            test_clear_loading_diagnostics,
            |mut commands: Commands| {
                commands.insert_resource(TestLoadingWatchdog {
                    started: Instant::now(),
                });
            },
        ),
    );

    app.add_systems(
        Update,
        (
            test_build_masks_during_loading
                .run_if(in_state(Screen::Loading).and_then(test_all_required_maps_terminal)),
            test_advance_collision_reload_gate.run_if(in_state(Screen::Loading)),
            test_detect_collision_failures
                .after(test_advance_collision_reload_gate)
                .run_if(in_state(Screen::Loading)),
            move |resource_handles: Res<alveus_asset_tracking::ResourceHandles>,
                  masks: Res<CollisionMasks>,
                  failures: Res<CollisionLoadFailures>,
                  stall: Option<Res<StallLoadingForTimeoutTest>>,
                  screen: Res<State<Screen>>,
                  mut next_screen: ResMut<NextState<Screen>>| {
                if stall.is_some() || *screen.get() != Screen::Loading {
                    return;
                }
                if failures.is_empty()
                    && resource_handles.is_all_done()
                    && collision_ready(&masks)
                {
                    next_screen.set(Screen::Gameplay);
                }
            },
            move |watchdog: Option<Res<TestLoadingWatchdog>>,
                  timing: Res<LoadingTiming>,
                  failures: Res<CollisionLoadFailures>,
                  screen: Res<State<Screen>>,
                  mut next_screen: ResMut<NextState<Screen>>| {
                if *screen.get() != Screen::Loading || failures.is_empty() {
                    return;
                }
                let Some(watchdog) = watchdog else {
                    return;
                };
                if watchdog.started.elapsed().as_secs_f32() < timing.failure_return_secs {
                    return;
                }
                next_screen.set(Screen::Title);
            },
            move |watchdog: Option<Res<TestLoadingWatchdog>>,
                  timing: Res<LoadingTiming>,
                  resource_handles: Res<alveus_asset_tracking::ResourceHandles>,
                  masks: Res<CollisionMasks>,
                  failures: Res<CollisionLoadFailures>,
                  level_assets: Option<Res<LevelAssets>>,
                  interior_assets: Option<Res<InteriorAssets>>,
                  mut timeout: ResMut<LoadingTimeoutDiagnostic>,
                  screen: Res<State<Screen>>,
                  mut next_screen: ResMut<NextState<Screen>>| {
                if *screen.get() != Screen::Loading || !failures.is_empty() {
                    return;
                }
                let Some(watchdog) = watchdog else {
                    return;
                };
                if watchdog.started.elapsed().as_secs_f32() < timing.timeout_secs {
                    return;
                }
                if resource_handles.is_all_done() && collision_ready(&masks) {
                    return;
                }
                let missing: Vec<_> = REQUIRED_COLLISION_KEYS
                    .iter()
                    .filter(|key| !masks.contains(**key))
                    .map(|key| format!("{key:?}"))
                    .collect();
                *timeout = LoadingTimeoutDiagnostic {
                    timed_out: true,
                    missing_keys: missing,
                    is_all_done: resource_handles.is_all_done(),
                    has_level_assets: level_assets.is_some(),
                    has_interior_assets: interior_assets.is_some(),
                };
                next_screen.set(Screen::Title);
            },
        ),
    );

    // Start on Title so Play routes through Loading when assets are still pending.
    app.insert_resource(NextState::Pending(Screen::Title));
    app.update();
    let _ = PlayClickEvent;
    app
}
