//! Loading failure / timeout diagnostics must return to Title without entering Gameplay.

use std::time::{Duration, Instant};

use alveus_app::Screen;
use alveus_collision::{
    COLLISION_LOAD_REASON_RECURSIVE_DEPENDENCY_FAILED, CollisionLoadFailures, CollisionMapKey,
    CollisionMasks,
};
use alveus_menus::PlayClickEvent;
use alveus_screens::{LoadingTimeoutDiagnostic, LoadingTiming};
use bevy::prelude::*;

mod common;

/// Deliberately invalid Tiled XML so the map loader reports Failed.
const CORRUPT_TMX: &[u8] = b"not a valid tiled map";

#[test]
fn missing_overview_map_records_failure_and_returns_to_title() {
    let mut app = common::loading_diagnostic_app(&[("maps/overview/map.tmx", CORRUPT_TMX)]);
    app.insert_resource(LoadingTiming {
        timeout_secs: 30.0,
        failure_return_secs: 0.05,
    });

    assert_eq!(
        *app.world().resource::<State<Screen>>().get(),
        Screen::Title
    );

    app.world_mut().trigger(PlayClickEvent);
    app.update();

    let deadline = Instant::now() + Duration::from_secs(5);
    let mut saw_failure = false;
    while Instant::now() < deadline {
        app.update();
        let failures = app.world().resource::<CollisionLoadFailures>();
        if !failures.is_empty() {
            saw_failure = true;
            assert!(
                failures.contains_key(CollisionMapKey::Overview),
                "expected Overview failure, got {:?}",
                failures.entries
            );
            assert_eq!(
                failures.entries[0].reason,
                COLLISION_LOAD_REASON_RECURSIVE_DEPENDENCY_FAILED
            );
            assert_eq!(failures.entries[0].asset_path, "maps/overview/map.tmx");
        }
        if *app.world().resource::<State<Screen>>().get() == Screen::Title && saw_failure {
            break;
        }
        assert_ne!(
            *app.world().resource::<State<Screen>>().get(),
            Screen::Gameplay,
            "failed load must not enter Gameplay"
        );
    }

    assert!(saw_failure, "expected CollisionLoadFailures entry");
    assert_eq!(
        *app.world().resource::<State<Screen>>().get(),
        Screen::Title,
        "should return to Title after failure grace period"
    );
    assert!(
        !app.world()
            .resource::<LoadingTimeoutDiagnostic>()
            .timed_out,
        "explicit failure must not be recorded as timeout"
    );
}

#[test]
fn loading_retry_clears_stale_failure_entries() {
    let mut app = common::loading_diagnostic_app(&[("maps/overview/map.tmx", CORRUPT_TMX)]);
    app.insert_resource(LoadingTiming {
        timeout_secs: 30.0,
        failure_return_secs: 0.05,
    });

    app.world_mut().trigger(PlayClickEvent);
    let deadline = Instant::now() + Duration::from_secs(5);
    while Instant::now() < deadline {
        app.update();
        if *app.world().resource::<State<Screen>>().get() == Screen::Title
            && !app.world().resource::<CollisionLoadFailures>().is_empty()
        {
            break;
        }
    }
    assert!(!app.world().resource::<CollisionLoadFailures>().is_empty());

    app.world_mut().trigger(PlayClickEvent);
    app.update();
    assert_eq!(
        *app.world().resource::<State<Screen>>().get(),
        Screen::Loading
    );

    for _ in 0..50 {
        app.update();
    }
    let failures = app.world().resource::<CollisionLoadFailures>();
    assert!(
        failures.entries.len() <= 1,
        "dedupe must keep a single Overview entry, got {}",
        failures.entries.len()
    );
}

#[test]
fn pending_assets_timeout_records_distinct_diagnostic() {
    let mut app = common::loading_diagnostic_app(&[]);
    app.insert_resource(LoadingTiming {
        timeout_secs: 0.05,
        failure_return_secs: 60.0,
    });

    app.world_mut().trigger(PlayClickEvent);
    app.update();

    let deadline = Instant::now() + Duration::from_secs(2);
    while Instant::now() < deadline {
        *app.world_mut().resource_mut::<CollisionMasks>() = CollisionMasks::default();
        app.update();
        match *app.world().resource::<State<Screen>>().get() {
            Screen::Title => {
                let timeout = app.world().resource::<LoadingTimeoutDiagnostic>();
                let failures = app.world().resource::<CollisionLoadFailures>();
                assert!(
                    timeout.timed_out,
                    "expected LoadingTimeoutDiagnostic.timed_out"
                );
                assert!(
                    failures.is_empty(),
                    "timeout path must not pretend to be an asset failure: {failures:?}"
                );
                assert!(
                    !timeout.missing_keys.is_empty(),
                    "timeout should list missing collision keys"
                );
                return;
            }
            Screen::Gameplay => {
                panic!("timeout should return to Title before Gameplay");
            }
            _ => {}
        }
    }

    panic!(
        "expected timeout return to Title; screen={:?}, failures={:?}, timeout={:?}",
        app.world().resource::<State<Screen>>().get(),
        app.world().resource::<CollisionLoadFailures>(),
        app.world().resource::<LoadingTimeoutDiagnostic>()
    );
}
