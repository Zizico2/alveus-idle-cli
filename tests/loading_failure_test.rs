//! Loading failure / timeout diagnostics must return to Title without entering Gameplay.

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use alveus_app::Screen;
use alveus_collision::{
    COLLISION_LOAD_REASON_RECURSIVE_DEPENDENCY_FAILED, CollisionLoadFailures, CollisionMapKey,
};
use alveus_menus::PlayClickEvent;
use alveus_screens::{LoadingTimeoutDiagnostic, LoadingTiming};
use bevy::prelude::*;

mod common;

/// Deliberately invalid Tiled XML so the map loader reports Failed.
const CORRUPT_TMX: &[u8] = b"not a valid tiled map";
/// Deliberately invalid PNG so the map loads but a recursive image dependency fails.
const CORRUPT_PNG: &[u8] = b"not a valid png";

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
            assert!(
                !failures.toast_message().contains('\n'),
                "toast copy must stay single-line"
            );
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
fn loading_retry_recovers_after_asset_is_repaired() {
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
    assert_eq!(
        *app.world().resource::<State<Screen>>().get(),
        Screen::Title
    );

    // Repair the fixture in the shared memory store, then Play again. Loading
    // must reload Failed handles — not just clear the diagnostic resource.
    let good_overview = std::fs::read(
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets/maps/overview/map.tmx"),
    )
    .expect("shipped overview map");
    app.world()
        .resource::<common::MemoryAssetStore>()
        .0
        .insert_asset(Path::new("maps/overview/map.tmx"), good_overview);

    app.world_mut().trigger(PlayClickEvent);
    app.update();
    assert_eq!(
        *app.world().resource::<State<Screen>>().get(),
        Screen::Loading
    );

    let deadline = Instant::now() + Duration::from_secs(5);
    while Instant::now() < deadline {
        app.update();
        if *app.world().resource::<State<Screen>>().get() == Screen::Gameplay {
            assert!(
                app.world()
                    .resource::<CollisionLoadFailures>()
                    .is_empty(),
                "successful retry must clear failure entries"
            );
            return;
        }
    }

    panic!(
        "expected Gameplay after repairing overview map; screen={:?}, failures={:?}",
        app.world().resource::<State<Screen>>().get(),
        app.world().resource::<CollisionLoadFailures>()
    );
}

#[test]
fn loading_retry_while_still_broken_records_failure_again() {
    assert_loading_retry_while_still_broken(&[("maps/overview/map.tmx", CORRUPT_TMX)]);
}

#[test]
fn loading_retry_with_broken_recursive_dependency_records_failure_again() {
    assert_loading_retry_while_still_broken(&[("maps/overview/sand_tile.png", CORRUPT_PNG)]);
}

fn assert_loading_retry_while_still_broken(replacements: &[(&str, &[u8])]) {
    let mut app = common::loading_diagnostic_app(replacements);
    // Long timeout: a permanently gated Failed must not be misreported as timeout.
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
    assert!(
        !app.world().resource::<CollisionLoadFailures>().is_empty(),
        "first Play must record an explicit collision load failure"
    );
    assert_eq!(
        *app.world().resource::<State<Screen>>().get(),
        Screen::Title
    );

    // Play again without repairing — reload stays Failed with no intermediate
    // non-Failed state. The gate must still complete so we re-record failure.
    app.world_mut().trigger(PlayClickEvent);
    app.update();
    assert_eq!(
        *app.world().resource::<State<Screen>>().get(),
        Screen::Loading
    );

    let deadline = Instant::now() + Duration::from_secs(5);
    let mut saw_failure_again = false;
    while Instant::now() < deadline {
        app.update();
        let timeout = app.world().resource::<LoadingTimeoutDiagnostic>();
        assert!(
            !timeout.timed_out,
            "still-broken retry must not fall through to the Loading timeout"
        );
        let failures = app.world().resource::<CollisionLoadFailures>();
        if !failures.is_empty() {
            saw_failure_again = true;
            assert!(
                failures.contains_key(CollisionMapKey::Overview),
                "expected Overview failure on retry, got {:?}",
                failures.entries
            );
        }
        if *app.world().resource::<State<Screen>>().get() == Screen::Title && saw_failure_again {
            return;
        }
        assert_ne!(
            *app.world().resource::<State<Screen>>().get(),
            Screen::Gameplay,
            "still-broken retry must not enter Gameplay"
        );
    }

    panic!(
        "expected Title with CollisionLoadFailures after still-broken retry; screen={:?}, failures={:?}, timeout={:?}",
        app.world().resource::<State<Screen>>().get(),
        app.world().resource::<CollisionLoadFailures>(),
        app.world().resource::<LoadingTimeoutDiagnostic>()
    );
}

#[test]
fn pending_assets_timeout_records_distinct_diagnostic() {
    let mut app = common::loading_diagnostic_app(&[]);
    app.insert_resource(common::StallLoadingForTimeoutTest);
    app.insert_resource(LoadingTiming {
        timeout_secs: 0.05,
        failure_return_secs: 60.0,
    });

    app.world_mut().trigger(PlayClickEvent);
    app.update();

    let deadline = Instant::now() + Duration::from_secs(2);
    while Instant::now() < deadline {
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
                let toast = timeout.player_message().expect("timeout toast");
                assert!(!toast.contains('\n'));
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
