//! A loading screen during which game assets are loaded if necessary.
//! This reduces stuttering, especially for audio on Wasm.

use std::collections::HashSet;
use std::time::Instant;

use bevy::prelude::*;
use bevy_ecs_tiled::prelude::TiledMapAsset;

use alveus_app::Screen;
use alveus_asset_tracking::ResourceHandles;
use alveus_collision::{
    CollisionMapKey, CollisionMasks, InteriorAssets, LevelAssets, REQUIRED_COLLISION_KEYS,
    build_all_collision_masks, collision_ready, warn_failed_collision_map_loads,
};
use alveus_configs::LOADING_TIMEOUT_SECS;
use alveus_theme::prelude::*;

/// Wall-clock start of the current Loading visit.
#[derive(Resource, Debug)]
struct LoadingWatchdog {
    started: Instant,
}

pub(super) fn plugin(app: &mut App) {
    app.add_systems(
        OnEnter(Screen::Loading),
        (spawn_loading_screen, insert_loading_watchdog),
    );
    app.add_systems(OnExit(Screen::Loading), remove_loading_watchdog);

    app.add_systems(
        Update,
        (
            build_collision_masks_during_loading,
            enter_gameplay_screen
                .after(build_collision_masks_during_loading)
                .before(loading_timeout_watchdog)
                .run_if(in_state(Screen::Loading).and_then(loading_complete)),
            loading_timeout_watchdog
                .after(build_collision_masks_during_loading)
                .run_if(in_state(Screen::Loading)),
        ),
    );
}

fn spawn_loading_screen(mut commands: Commands) {
    commands.spawn((
        widget::ui_root("Loading Screen"),
        DespawnOnExit(Screen::Loading),
        children![widget::label("Loading...")],
    ));
}

fn insert_loading_watchdog(mut commands: Commands) {
    commands.insert_resource(LoadingWatchdog {
        started: Instant::now(),
    });
}

fn remove_loading_watchdog(mut commands: Commands) {
    commands.remove_resource::<LoadingWatchdog>();
}

fn build_collision_masks_during_loading(
    mut masks: ResMut<CollisionMasks>,
    map_assets: Res<Assets<TiledMapAsset>>,
    asset_server: Res<AssetServer>,
    level_assets: Option<Res<LevelAssets>>,
    interior_assets: Option<Res<InteriorAssets>>,
    mut warned: Local<HashSet<CollisionMapKey>>,
) {
    let (Some(level_assets), Some(interior_assets)) = (level_assets, interior_assets) else {
        return;
    };

    if !collision_ready(&masks) {
        build_all_collision_masks(&mut masks, &map_assets, &level_assets, &interior_assets);
    }

    warn_failed_collision_map_loads(
        &map_assets,
        &level_assets,
        &interior_assets,
        &asset_server,
        &mut warned,
    );
}

fn loading_complete(resource_handles: Res<ResourceHandles>, masks: Res<CollisionMasks>) -> bool {
    resource_handles.is_all_done() && collision_ready(&masks)
}

fn enter_gameplay_screen(mut next_screen: ResMut<NextState<Screen>>) {
    next_screen.set(Screen::Gameplay);
}

fn loading_timeout_watchdog(
    watchdog: Option<Res<LoadingWatchdog>>,
    resource_handles: Res<ResourceHandles>,
    masks: Res<CollisionMasks>,
    level_assets: Option<Res<LevelAssets>>,
    interior_assets: Option<Res<InteriorAssets>>,
    mut next_screen: ResMut<NextState<Screen>>,
) {
    let Some(watchdog) = watchdog else {
        return;
    };
    if watchdog.started.elapsed().as_secs_f32() < LOADING_TIMEOUT_SECS {
        return;
    }
    // Success path owns the transition; do not race Title when load just completed.
    if resource_handles.is_all_done() && collision_ready(&masks) {
        return;
    }

    let missing: Vec<_> = REQUIRED_COLLISION_KEYS
        .iter()
        .filter(|key| !masks.contains(**key))
        .map(|key| format!("{key:?}"))
        .collect();
    error!(
        "Loading timed out after {LOADING_TIMEOUT_SECS}s; returning to Title. \
         is_all_done={}, LevelAssets={}, InteriorAssets={}, collision_ready={}, missing_keys=[{}]",
        resource_handles.is_all_done(),
        level_assets.is_some(),
        interior_assets.is_some(),
        collision_ready(&masks),
        missing.join(", ")
    );
    next_screen.set(Screen::Title);
}
