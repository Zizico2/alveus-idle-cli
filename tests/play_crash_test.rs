use bevy::prelude::*;
use bevy::state::app::StatesPlugin;
use alveus_idle_cli::screens::Screen;
use alveus_idle_cli::menus::PlayClickEvent;
use alveus_idle_cli::stats::{StatsPlugin, SavePath};

fn is_valid_save_format(content: &str) -> bool {
    if let Ok(ron::Value::Map(map)) = ron::from_str::<ron::Value>(content) {
        for key in map.keys() {
            if let ron::Value::String(s) = key {
                if s == "resources" || s == "entities" {
                    return true;
                }
            }
        }
    }
    false
}

#[test]
fn test_print_ron_parsing() {
    let invalid_ron = "(timestamp:1780883482,animals:{\"georgie\":(hunger:449,happiness:412)},enclosures:{})";
    assert!(!is_valid_save_format(invalid_ron), "Invalid format should not be valid");

    let mock_valid_ron = "(resources: [], entities: [])";
    assert!(is_valid_save_format(mock_valid_ron), "Valid format should be valid");
}

#[test]
fn test_play_crash_on_invalid_save() {
    let test_save_path = "invalid_test_save.ron";
    let _ = std::fs::remove_file(test_save_path);
    std::fs::write(test_save_path, "(timestamp:1780883482,animals:{\"georgie\":(hunger:449,happiness:412)},enclosures:{})").unwrap();

    let mut app = App::new();
    app.add_plugins(StatesPlugin);
    app.init_state::<Screen>();
    app.add_plugins(MinimalPlugins);
    app.init_resource::<ButtonInput<KeyCode>>();
    app.init_resource::<alveus_idle_cli::asset_tracking::ResourceHandles>();
    app.insert_resource(SavePath(test_save_path.to_string()));

    // Manually register only the play click observer and stats plugin
    app.add_observer(alveus_idle_cli::menus::main::handle_play_click);
    app.add_plugins(StatsPlugin);

    // Initialize state transitions
    app.update();

    // Trigger PlayClickEvent directly
    app.world_mut().trigger(PlayClickEvent);

    // Update app: should transition to Gameplay, load invalid save, and crash
    app.update();

    // Cleanup
    let _ = std::fs::remove_file(test_save_path);
}

#[test]
fn test_play_without_save() {
    let mut app = App::new();
    app.add_plugins(StatesPlugin);
    app.init_state::<Screen>();
    app.add_plugins(MinimalPlugins);
    app.init_resource::<ButtonInput<KeyCode>>();
    app.init_resource::<alveus_idle_cli::asset_tracking::ResourceHandles>();

    // Manually register only the play click observer and stats plugin
    app.add_observer(alveus_idle_cli::menus::main::handle_play_click);
    app.add_plugins(StatsPlugin);

    // Initialize state transitions
    app.update();

    // Trigger PlayClickEvent directly
    app.world_mut().trigger(PlayClickEvent);

    // Update app: should transition to Gameplay screen and initialize stats without crashing
    app.update();

    // Assert that the screen state is gameplay and animal entities were spawned
    assert_eq!(*app.world().resource::<State<Screen>>().get(), Screen::Gameplay);

    let animal_count = app.world_mut().query::<&alveus_idle_cli::stats::AnimalId>().iter(app.world()).count();
    assert_eq!(animal_count, 4, "Should initialize default 4 animals when save does not exist");
}
