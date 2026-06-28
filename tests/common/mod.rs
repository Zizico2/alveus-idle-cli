use alveus_idle_cli::headless::CommandPlugin;
use alveus_idle_cli::screens::Screen;
use alveus_idle_cli::stats::{SavePath, StatsPlugin};
use bevy::prelude::*;
use bevy::state::app::StatesPlugin;

pub fn minimal_stats_app(save_path: &str) -> App {
    let mut app = App::new();
    app.add_plugins(StatesPlugin);
    app.init_state::<Screen>();
    app.add_plugins(MinimalPlugins);
    app.add_plugins(AssetPlugin::default());
    app.init_resource::<ButtonInput<KeyCode>>();
    app.insert_resource(SavePath(save_path.to_string()));
    app.add_plugins((StatsPlugin, CommandPlugin));
    app.insert_resource(NextState::Pending(Screen::Gameplay));
    app.update();
    app
}

pub fn cleanup_save(save_path: &str) {
    let _ = std::fs::remove_file(save_path);
}
