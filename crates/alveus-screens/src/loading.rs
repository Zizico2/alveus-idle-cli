//! A loading screen during which game assets are loaded if necessary.
//! This reduces stuttering, especially for audio on Wasm.

use std::time::Instant;

use bevy::prelude::*;
use bevy_ecs_tiled::prelude::TiledMapAsset;

use alveus_app::Screen;
use alveus_asset_tracking::ResourceHandles;
use alveus_collision::{
    CollisionLoadFailures, CollisionMasks, InteriorAssets, LevelAssets, REQUIRED_COLLISION_KEYS,
    RequiredCollisionMapState, build_all_collision_masks, collision_ready,
    record_failed_collision_map_loads, required_collision_handles, required_collision_map_state,
    required_collision_maps_terminal,
};
use alveus_configs::LOADING_TIMEOUT_SECS;
use alveus_theme::prelude::*;

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
}

impl Default for LoadingTiming {
    fn default() -> Self {
        Self {
            timeout_secs: LOADING_TIMEOUT_SECS,
        }
    }
}

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

    pub fn fatal_message(&self) -> Option<String> {
        if !self.timed_out {
            return None;
        }
        Some("Loading timed out. Restart the game.".to_string())
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
    app.add_systems(OnEnter(Screen::FatalError), spawn_fatal_error_screen);

    app.add_systems(
        Update,
        (
            build_collision_masks_during_loading
                .run_if(in_state(Screen::Loading).and_then(all_required_collision_maps_loaded)),
            detect_collision_load_failures_during_loading,
            enter_fatal_error_on_collision_failure,
            enter_gameplay_screen,
            loading_timeout_watchdog,
        )
            .chain()
            .run_if(in_state(Screen::Loading)),
    );
}

fn clear_loading_diagnostics(
    mut failures: ResMut<CollisionLoadFailures>,
    mut timeout: ResMut<LoadingTimeoutDiagnostic>,
) {
    failures.clear();
    timeout.clear();
}

fn spawn_loading_screen(mut commands: Commands) {
    commands.spawn((
        widget::ui_root("Loading Screen"),
        DespawnOnExit(Screen::Loading),
        children![(
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

/// True once every required root map and its dependencies have loaded.
fn all_required_collision_maps_loaded(
    asset_server: Res<AssetServer>,
    level_assets: Res<LevelAssets>,
    interior_assets: Res<InteriorAssets>,
) -> bool {
    let handles = required_collision_handles(&level_assets, &interior_assets);
    required_collision_maps_terminal(&asset_server, &handles)
        && handles.iter().all(|(_, handle)| {
            required_collision_map_state(&asset_server, handle) == RequiredCollisionMapState::Loaded
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

fn detect_collision_load_failures_during_loading(
    asset_server: Res<AssetServer>,
    level_assets: Res<LevelAssets>,
    interior_assets: Res<InteriorAssets>,
    mut failures: ResMut<CollisionLoadFailures>,
) {
    let handles = required_collision_handles(&level_assets, &interior_assets);
    record_failed_collision_map_loads(&asset_server, &handles, &mut failures);
}

fn enter_fatal_error_on_collision_failure(
    failures: Res<CollisionLoadFailures>,
    mut next_screen: ResMut<NextState<Screen>>,
) {
    if !failures.is_empty() {
        next_screen.set(Screen::FatalError);
    }
}

fn enter_gameplay_screen(
    resource_handles: Res<ResourceHandles>,
    masks: Res<CollisionMasks>,
    failures: Res<CollisionLoadFailures>,
    mut next_screen: ResMut<NextState<Screen>>,
) {
    let loading_complete =
        failures.is_empty() && resource_handles.is_all_done() && collision_ready(&masks);
    if loading_complete {
        // Fetch and write the transition only after failure detection has run in
        // this frame; a run condition may be evaluated before that mutation.
        // This prevents stale success from racing a newly observed load failure.
        next_screen.set(Screen::Gameplay);
    }
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
    // Explicit asset failures transition directly to FatalError.
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
        "Loading timed out after {}s; entering FatalError. \
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
    next_screen.set(Screen::FatalError);
}

fn spawn_fatal_error_screen(
    mut commands: Commands,
    failures: Res<CollisionLoadFailures>,
    timeout: Res<LoadingTimeoutDiagnostic>,
) {
    let message = if !failures.is_empty() {
        failures.loading_detail_message()
    } else {
        timeout
            .fatal_message()
            .unwrap_or_else(|| "A required game asset could not be loaded.".to_string())
    };

    commands.spawn((
        widget::ui_root("Fatal Error Screen"),
        DespawnOnExit(Screen::FatalError),
        children![
            (
                Name::new("Fatal Error Title"),
                Text::new("GAME CANNOT START"),
                TextFont::from_font_size(32.0),
                TextColor(Color::srgb(0.95, 0.35, 0.35)),
            ),
            (
                Name::new("Fatal Error Detail"),
                Text::new(message),
                TextFont::from_font_size(20.0),
                TextColor(ui_palette::LABEL_TEXT),
                Node {
                    max_width: Val::Px(640.0),
                    ..default()
                },
                TextLayout {
                    justify: Justify::Center,
                    linebreak: LineBreak::WordOrCharacter,
                },
            ),
            (
                Name::new("Fatal Error Restart Instruction"),
                Text::new("Restart the game after fixing the installation or asset files."),
                TextFont::from_font_size(16.0),
                TextColor(ui_palette::LABEL_TEXT),
            ),
        ],
    ));
}
