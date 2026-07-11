//! Required loading failures are terminal for the current process.

use std::time::{Duration, Instant};

use alveus_app::Screen;
use alveus_menus::PlayClickEvent;
use alveus_screens::LoadingTiming;
use bevy::prelude::*;

mod common;

/// Deliberately invalid Tiled XML so the map loader reports Failed.
const CORRUPT_TMX: &[u8] = b"not a valid tiled map";
/// Deliberately invalid PNG so the map loads but a recursive image dependency fails.
const CORRUPT_PNG: &[u8] = b"not a valid png";

#[test]
fn required_root_map_failure_enters_fatal_error_without_gameplay() {
    let mut app = common::loading_diagnostic_app(&[("maps/overview/map.tmx", CORRUPT_TMX)]);
    app.insert_resource(LoadingTiming { timeout_secs: 30.0 });

    app.world_mut().trigger(PlayClickEvent);

    wait_for_fatal_error(&mut app);
}

#[test]
fn required_recursive_dependency_failure_enters_fatal_error_without_gameplay() {
    let mut app = common::loading_diagnostic_app(&[("maps/overview/sand_tile.png", CORRUPT_PNG)]);
    app.insert_resource(LoadingTiming { timeout_secs: 30.0 });

    app.world_mut().trigger(PlayClickEvent);

    wait_for_fatal_error(&mut app);
}

#[test]
fn pending_assets_timeout_enters_fatal_error() {
    let mut app = common::loading_diagnostic_app(&[]);
    app.insert_resource(common::StallLoadingForTimeoutTest);
    app.insert_resource(LoadingTiming { timeout_secs: 0.05 });

    app.world_mut().trigger(PlayClickEvent);

    wait_for_fatal_error(&mut app);
}

fn wait_for_fatal_error(app: &mut App) {
    let deadline = Instant::now() + Duration::from_secs(5);
    while Instant::now() < deadline {
        app.update();
        match *app.world().resource::<State<Screen>>().get() {
            Screen::FatalError => return,
            Screen::Gameplay | Screen::InRoom(_) => {
                panic!("failed Loading must not enter gameplay");
            }
            _ => {}
        }
    }

    panic!(
        "expected FatalError; screen={:?}",
        app.world().resource::<State<Screen>>().get(),
    );
}
