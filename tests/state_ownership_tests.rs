use std::fs;
use std::path::{Path, PathBuf};

use alveus_app::{Menu, Pause, Screen};
use alveus_command::CommandPlugin;
use alveus_interaction::InteractionPlugin;
use bevy::prelude::*;
use bevy::state::app::StatesPlugin;

#[test]
fn app_plugin_supports_representative_minimal_consumers() {
    let mut app = App::new();
    app.add_plugins(StatesPlugin);
    app.add_plugins(MinimalPlugins);
    app.add_plugins(alveus_app::plugin);
    app.add_plugins((InteractionPlugin, CommandPlugin));

    app.update();

    assert_eq!(
        *app.world().resource::<State<Screen>>().get(),
        Screen::Splash
    );
    assert_eq!(*app.world().resource::<State<Menu>>().get(), Menu::None);
    assert_eq!(*app.world().resource::<State<Pause>>().get(), Pause(false));
}

#[test]
fn app_plugin_is_the_only_production_app_state_owner() {
    let workspace = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let mut initializers = Vec::new();

    collect_state_initializers(&workspace, &workspace.join("crates"), &mut initializers);
    collect_state_initializers(&workspace, &workspace.join("src"), &mut initializers);
    initializers.sort();

    assert_eq!(
        initializers,
        [
            "crates/alveus-app/src/lib.rs:init_state::<Menu>()",
            "crates/alveus-app/src/lib.rs:init_state::<Pause>()",
            "crates/alveus-app/src/lib.rs:init_state::<Screen>()",
        ],
        "app-wide states must be initialized only by alveus_app::plugin"
    );
}

#[test]
fn production_gameplay_avoids_exclusive_world_dispatch_patterns() {
    let workspace = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let banned = [
        "_in_world",
        "PendingGameCommands",
        "apply_pending_game_commands",
        "apply_game_command",
        "run_system_once",
        "run_system_cached",
        "register_system",
        "SystemId",
    ];
    let scan_roots = [workspace.join("crates"), workspace.join("src")];
    let allowlist_paths = ["crates/alveus-asset-tracking/", "FromWorld::from_world"];

    for root in &scan_roots {
        collect_banned_patterns(root, &workspace, &banned, &allowlist_paths);
    }
}

fn collect_banned_patterns(
    directory: &Path,
    workspace: &Path,
    banned: &[&str],
    allowlist_paths: &[&str],
) {
    for entry in fs::read_dir(directory).expect("read source directory") {
        let path = entry.expect("source entry").path();
        if path.is_dir() {
            collect_banned_patterns(&path, workspace, banned, allowlist_paths);
            continue;
        }
        if path.extension().and_then(|extension| extension.to_str()) != Some("rs") {
            continue;
        }
        let relative = path
            .strip_prefix(workspace)
            .expect("workspace source path")
            .to_string_lossy();
        if allowlist_paths
            .iter()
            .any(|allowed| relative.contains(allowed))
        {
            continue;
        }
        if relative.starts_with("crates/alveus-command/")
            || relative.starts_with("crates/alveus-interaction/")
            || relative.starts_with("crates/alveus-screens/")
            || relative.starts_with("crates/alveus-stats/")
            || relative.starts_with("crates/alveus-world/src/room.rs")
            || relative.starts_with("crates/alveus-hud/")
        {
            let source = fs::read_to_string(&path).expect("read Rust source");
            for pattern in banned {
                assert!(
                    !source.contains(pattern),
                    "{relative} must not contain banned pattern `{pattern}`"
                );
            }
            if source.contains("world: &mut World") || source.contains("&mut World)") {
                let lines: Vec<_> = source
                    .lines()
                    .filter(|line| {
                        (line.contains("world: &mut World") || line.contains("&mut World)"))
                            && !line.trim_start().starts_with("//")
                    })
                    .collect();
                assert!(
                    lines.is_empty(),
                    "{relative} must not use &mut World in gameplay paths: {lines:?}"
                );
            }
        }
    }
}

fn collect_state_initializers(workspace: &Path, directory: &Path, found: &mut Vec<String>) {
    for entry in fs::read_dir(directory).expect("read source directory") {
        let path = entry.expect("source entry").path();
        if path.is_dir() {
            collect_state_initializers(workspace, &path, found);
            continue;
        }
        if path.extension().and_then(|extension| extension.to_str()) != Some("rs") {
            continue;
        }

        let source = fs::read_to_string(&path).expect("read Rust source");
        for state in ["Screen", "Menu", "Pause"] {
            let initializer = format!("init_state::<{state}>()");
            for _ in source.match_indices(&initializer) {
                let relative = path.strip_prefix(workspace).expect("workspace source path");
                found.push(format!("{}:{initializer}", relative.display()));
            }
        }
    }
}
