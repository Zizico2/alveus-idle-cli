//! A loading screen during which game assets are loaded if necessary.
//! This reduces stuttering, especially for audio on Wasm.

use std::time::Instant;

use bevy::prelude::*;
use bevy_ecs_tiled::prelude::TiledMapAsset;

use alveus_app::Screen;
use alveus_asset_tracking::ResourceHandles;
use alveus_collision::{
    CollisionLoadFailures, CollisionMasks, CollisionReloadGate, InteriorAssets, LevelAssets,
    REQUIRED_COLLISION_KEYS, RequiredCollisionMapHandles, advance_collision_reload_gate,
    build_all_collision_masks, collision_ready, record_failed_collision_map_loads,
    reload_failed_collision_maps, required_collision_handles,
};
use alveus_configs::{LOADING_FAILURE_RETURN_SECS, LOADING_TIMEOUT_SECS};
use alveus_theme::prelude::*;
use alveus_world::toast::{DismissToastEvent, TriggerToastEvent};

/// Wall-clock start of the current Loading visit.
#[derive(Resource, Debug)]
struct LoadingWatchdog {
    started: Instant,
}

/// Overridable loading deadlines (defaults from [`alveus_configs`]).
///
/// Tests may shorten these; production uses the config constants.
#[derive(Resource, Debug, Clone, Copy)]
pub struct LoadingTiming {
    pub timeout_secs: f32,
    pub failure_return_secs: f32,
}

impl Default for LoadingTiming {
    fn default() -> Self {
        Self {
            timeout_secs: LOADING_TIMEOUT_SECS,
            failure_return_secs: LOADING_FAILURE_RETURN_SECS,
        }
    }
}

/// Marker for the Loading screen status label (updated on failure).
#[derive(Component)]
struct LoadingStatusLabel;

/// Distinct from [`CollisionLoadFailures`]: pending assets timed out without an
/// explicit Failed load state.
#[derive(Resource, Reflect, Debug, Clone, Default)]
#[reflect(Resource)]
pub struct LoadingTimeoutDiagnostic {
    pub timed_out: bool,
    pub missing_keys: Vec<String>,
    pub is_all_done: bool,
    pub has_level_assets: bool,
    pub has_interior_assets: bool,
}

impl LoadingTimeoutDiagnostic {
    pub fn clear(&mut self) {
        *self = Self::default();
    }

    pub fn player_message(&self) -> Option<String> {
        if !self.timed_out {
            return None;
        }
        // Keep toast-sized; missing keys stay in this resource and the error log.
        Some("Loading timed out. Returning to title…".to_string())
    }
}

pub(super) fn plugin(app: &mut App) {
    app.init_resource::<LoadingTimeoutDiagnostic>()
        .init_resource::<LoadingTiming>()
        .register_type::<LoadingTimeoutDiagnostic>();

    app.add_systems(
        OnEnter(Screen::Loading),
        (
            clear_loading_diagnostics,
            spawn_loading_screen,
            insert_loading_watchdog,
        )
            .chain(),
    );
    app.add_systems(OnExit(Screen::Loading), remove_loading_watchdog);

    app.add_systems(
        Update,
        (
            // Only borrow `Assets<TiledMapAsset>` once every required map is terminal.
            // Holding that resource while any load is still pending can stall Bevy in
            // `Loading` and hide `Failed` from failure detection.
            build_collision_masks_during_loading
                .run_if(in_state(Screen::Loading).and_then(all_required_collision_maps_terminal)),
            advance_collision_reload_gate_during_loading.run_if(in_state(Screen::Loading)),
            detect_collision_load_failures_during_loading
                .after(advance_collision_reload_gate_during_loading)
                .run_if(in_state(Screen::Loading)),
            update_loading_status_label
                .after(detect_collision_load_failures_during_loading)
                .run_if(in_state(Screen::Loading)),
            enter_gameplay_screen
                .after(build_collision_masks_during_loading)
                .after(detect_collision_load_failures_during_loading)
                .before(loading_failure_return)
                .before(loading_timeout_watchdog)
                .run_if(in_state(Screen::Loading).and_then(loading_complete)),
            loading_failure_return
                .after(detect_collision_load_failures_during_loading)
                .before(loading_timeout_watchdog)
                .run_if(in_state(Screen::Loading)),
            loading_timeout_watchdog
                .after(detect_collision_load_failures_during_loading)
                .run_if(in_state(Screen::Loading)),
        ),
    );
}

fn clear_loading_diagnostics(
    mut failures: ResMut<CollisionLoadFailures>,
    mut timeout: ResMut<LoadingTimeoutDiagnostic>,
    mut gate: ResMut<CollisionReloadGate>,
    mut failed_messages: ResMut<Messages<bevy::asset::AssetLoadFailedEvent<TiledMapAsset>>>,
    asset_server: Res<AssetServer>,
    required: Res<RequiredCollisionMapHandles>,
    level_assets: Option<Res<LevelAssets>>,
    interior_assets: Option<Res<InteriorAssets>>,
    mut commands: Commands,
) {
    // Drop failure events from earlier attempts so they cannot immediately
    // complete the gate for this Loading visit's reload.
    failed_messages.clear();

    let handles = required_collision_handles(
        &required,
        level_assets.as_deref(),
        interior_assets.as_deref(),
    );
    // Fresh Loading attempt: drop stale diagnostics and re-request any Failed maps
    // so Play can recover after assets are repaired.
    reload_failed_collision_maps(&asset_server, &handles, &mut gate);
    failures.clear();
    timeout.clear();
    commands.trigger(DismissToastEvent);
}

fn spawn_loading_screen(mut commands: Commands) {
    commands.spawn((
        widget::ui_root("Loading Screen"),
        DespawnOnExit(Screen::Loading),
        children![(
            LoadingStatusLabel,
            Name::new("Loading Status"),
            Text::new("Loading..."),
            TextFont::from_font_size(24.0),
            TextColor(ui_palette::LABEL_TEXT),
            Node {
                max_width: Val::Px(640.0),
                ..default()
            },
            TextLayout {
                justify: Justify::Center,
                linebreak: LineBreak::WordOrCharacter,
            },
        )],
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

/// True once every required map has reached Loaded or Failed (not still pending).
///
/// Used to delay borrowing `Assets<TiledMapAsset>` until Bevy can finish marking
/// failed loads — holding that resource mid-load can stall forever in `Loading`.
fn all_required_collision_maps_terminal(
    asset_server: Res<AssetServer>,
    required: Res<RequiredCollisionMapHandles>,
    level_assets: Option<Res<LevelAssets>>,
    interior_assets: Option<Res<InteriorAssets>>,
) -> bool {
    use bevy::asset::RecursiveDependencyLoadState;

    let handles = required_collision_handles(
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

fn build_collision_masks_during_loading(
    mut masks: ResMut<CollisionMasks>,
    map_assets: Res<Assets<TiledMapAsset>>,
    level_assets: Option<Res<LevelAssets>>,
    interior_assets: Option<Res<InteriorAssets>>,
) {
    if let (Some(level_assets), Some(interior_assets)) = (level_assets, interior_assets)
        && !collision_ready(&masks)
    {
        build_all_collision_masks(&mut masks, &map_assets, &level_assets, &interior_assets);
    }
}

fn advance_collision_reload_gate_during_loading(
    asset_server: Res<AssetServer>,
    required: Res<RequiredCollisionMapHandles>,
    level_assets: Option<Res<LevelAssets>>,
    interior_assets: Option<Res<InteriorAssets>>,
    mut gate: ResMut<CollisionReloadGate>,
    mut failed_events: MessageReader<bevy::asset::AssetLoadFailedEvent<TiledMapAsset>>,
) {
    let handles = required_collision_handles(
        &required,
        level_assets.as_deref(),
        interior_assets.as_deref(),
    );
    advance_collision_reload_gate(&asset_server, &handles, &mut gate, failed_events.read());
}

fn detect_collision_load_failures_during_loading(
    asset_server: Res<AssetServer>,
    required: Res<RequiredCollisionMapHandles>,
    level_assets: Option<Res<LevelAssets>>,
    interior_assets: Option<Res<InteriorAssets>>,
    mut failures: ResMut<CollisionLoadFailures>,
    gate: Res<CollisionReloadGate>,
) {
    let handles = required_collision_handles(
        &required,
        level_assets.as_deref(),
        interior_assets.as_deref(),
    );
    record_failed_collision_map_loads(&asset_server, &handles, &mut failures, &gate);
}

fn update_loading_status_label(
    failures: Res<CollisionLoadFailures>,
    mut labels: Query<&mut Text, With<LoadingStatusLabel>>,
) {
    if !failures.is_changed() || failures.is_empty() {
        return;
    }
    let message = failures.loading_detail_message();
    for mut text in &mut labels {
        text.0 = message.clone();
    }
}

fn loading_complete(
    resource_handles: Res<ResourceHandles>,
    masks: Res<CollisionMasks>,
    failures: Res<CollisionLoadFailures>,
) -> bool {
    failures.is_empty() && resource_handles.is_all_done() && collision_ready(&masks)
}

fn enter_gameplay_screen(mut next_screen: ResMut<NextState<Screen>>) {
    next_screen.set(Screen::Gameplay);
}

fn loading_failure_return(
    watchdog: Option<Res<LoadingWatchdog>>,
    timing: Res<LoadingTiming>,
    failures: Res<CollisionLoadFailures>,
    mut next_screen: ResMut<NextState<Screen>>,
) {
    if failures.is_empty() {
        return;
    }
    let Some(watchdog) = watchdog else {
        return;
    };
    if watchdog.started.elapsed().as_secs_f32() < timing.failure_return_secs {
        return;
    }
    next_screen.set(Screen::Title);
}

fn loading_timeout_watchdog(
    watchdog: Option<Res<LoadingWatchdog>>,
    timing: Res<LoadingTiming>,
    resource_handles: Res<ResourceHandles>,
    masks: Res<CollisionMasks>,
    failures: Res<CollisionLoadFailures>,
    level_assets: Option<Res<LevelAssets>>,
    interior_assets: Option<Res<InteriorAssets>>,
    mut timeout: ResMut<LoadingTimeoutDiagnostic>,
    mut next_screen: ResMut<NextState<Screen>>,
) {
    // Explicit asset failures use the shorter failure-return path.
    if !failures.is_empty() {
        return;
    }

    let Some(watchdog) = watchdog else {
        return;
    };
    if watchdog.started.elapsed().as_secs_f32() < timing.timeout_secs {
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
        "Loading timed out after {}s; returning to Title. \
         is_all_done={}, LevelAssets={}, InteriorAssets={}, collision_ready={}, missing_keys=[{}]",
        timing.timeout_secs,
        resource_handles.is_all_done(),
        level_assets.is_some(),
        interior_assets.is_some(),
        collision_ready(&masks),
        missing.join(", ")
    );

    *timeout = LoadingTimeoutDiagnostic {
        timed_out: true,
        missing_keys: missing,
        is_all_done: resource_handles.is_all_done(),
        has_level_assets: level_assets.is_some(),
        has_interior_assets: interior_assets.is_some(),
    };
    next_screen.set(Screen::Title);
}

/// Surface the last loading diagnostic on Title so windowed players see it.
pub(super) fn surface_loading_diagnostic_on_title(
    mut commands: Commands,
    failures: Res<CollisionLoadFailures>,
    timeout: Res<LoadingTimeoutDiagnostic>,
) {
    if !failures.is_empty() {
        // Toast is fixed 240×60 — use the short summary; details stay on Loading/logs.
        commands.trigger(TriggerToastEvent::presence(failures.toast_message()));
        return;
    }
    if let Some(message) = timeout.player_message() {
        commands.trigger(TriggerToastEvent::presence(message));
    }
}
