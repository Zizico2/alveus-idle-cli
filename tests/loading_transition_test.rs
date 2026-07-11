//! Title → Loading → Gameplay must complete within a wall-clock budget.
//!
//! Reproduces the "stuck on Loading..." hang when collision masks / map assets
//! never become ready (e.g. after Epic 2 interior tileset changes).

use std::time::{Duration, Instant};

use alveus_app::Screen;
use alveus_asset_tracking::ResourceHandles;
use alveus_collision::{
    CollisionMasks, InteriorAssets, LevelAssets, REQUIRED_COLLISION_KEYS, collision_ready,
};
use alveus_menus::PlayClickEvent;
use bevy::prelude::*;

mod common;

fn loading_diagnostic(app: &App) -> String {
    let is_all_done = app.world().resource::<ResourceHandles>().is_all_done();
    let has_level = app.world().get_resource::<LevelAssets>().is_some();
    let has_interior = app.world().get_resource::<InteriorAssets>().is_some();
    let masks = app.world().resource::<CollisionMasks>();
    let missing: Vec<_> = REQUIRED_COLLISION_KEYS
        .iter()
        .filter(|key| !masks.contains(**key))
        .map(|key| format!("{key:?}"))
        .collect();
    format!(
        "is_all_done={is_all_done}, LevelAssets={has_level}, InteriorAssets={has_interior}, \
         collision_ready={}, missing_keys=[{}]",
        collision_ready(masks),
        missing.join(", ")
    )
}

#[test]
fn play_reaches_gameplay_within_five_seconds() {
    let mut app = common::loading_transition_app();

    assert_eq!(
        *app.world().resource::<State<Screen>>().get(),
        Screen::Title,
        "test harness should start on Title"
    );

    app.world_mut().trigger(PlayClickEvent);
    app.update();

    let screen = *app.world().resource::<State<Screen>>().get();
    assert!(
        matches!(screen, Screen::Loading | Screen::Gameplay),
        "Play should enter Loading or Gameplay, got {screen:?}; {}",
        loading_diagnostic(&app)
    );

    let deadline = Instant::now() + Duration::from_secs(5);
    while Instant::now() < deadline {
        if *app.world().resource::<State<Screen>>().get() == Screen::Gameplay {
            return;
        }
        app.update();
    }

    let screen = *app.world().resource::<State<Screen>>().get();
    panic!(
        "stuck on {screen:?} after 5s waiting for Loading → Gameplay; {}",
        loading_diagnostic(&app)
    );
}
