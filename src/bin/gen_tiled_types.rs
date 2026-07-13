//! Standalone tool that regenerates `assets/maps/overview/tiled_types.json`.
//!
//! The Tiled user-property type export is otherwise a side-effect of a normal
//! game run. This binary builds a minimal app (no render stack), registers every
//! Tiled-facing Reflect type via the shared `register_agent_types`, and calls
//! `export_types` directly — avoiding `TiledPlugin` / `TilemapPlugin`, which
//! require a `RenderApp` when the `render` feature is enabled.
//!
//! Run with: `cargo run --bin gen_tiled_types`

use bevy::prelude::*;

fn main() {
    let path = std::env::current_dir()
        .expect("current dir")
        .join("assets")
        .join("maps")
        .join("overview")
        .join("tiled_types.json");

    let mut app = App::new();
    alveus_reflect::register_agent_types(&mut app);
    alveus_world::level::export_tiled_types(&app, &path);

    println!("Wrote {}", path.display());
}
