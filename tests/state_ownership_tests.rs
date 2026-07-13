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
