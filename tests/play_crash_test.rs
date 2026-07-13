use alveus_app::Screen;
use alveus_screens::begin_play_in_world;
use alveus_stats::{SavePath, StatsPlugin};
use alveus_types::Stat;
use bevy::prelude::*;
use bevy::state::app::StatesPlugin;
use moonshine_save::prelude::SaveWorld;
use moonshine_save::save::TriggerSave;

fn is_valid_save_format(content: &str) -> bool {
    if let Ok(ron::Value::Map(map)) = ron::from_str::<ron::Value>(content) {
        for key in map.keys() {
            if let ron::Value::String(s) = key
                && (s == "resources" || s == "entities")
            {
                return true;
            }
        }
    }
    false
}

#[test]
fn test_print_ron_parsing() {
    let invalid_ron =
        "(timestamp:1780883482,animals:{\"georgie\":(hunger:449,happiness:412)},enclosures:{})";
    assert!(
        !is_valid_save_format(invalid_ron),
        "Invalid format should not be valid"
    );

    let mock_valid_ron = "(resources: [], entities: [])";
    assert!(
        is_valid_save_format(mock_valid_ron),
        "Valid format should be valid"
    );
}

#[test]
fn test_play_crash_on_invalid_save() {
    let test_save_path = "invalid_test_save.ron";
    let _ = std::fs::remove_file(test_save_path);
    std::fs::write(
        test_save_path,
        "(timestamp:1780883482,animals:{\"georgie\":(hunger:449,happiness:412)},enclosures:{})",
    )
    .unwrap();

    let mut app = App::new();
    app.add_plugins(StatesPlugin);
    app.add_plugins(alveus_app::plugin);
    app.add_plugins(MinimalPlugins);
    app.add_plugins(AssetPlugin::default());
    app.init_resource::<ButtonInput<KeyCode>>();
    app.init_resource::<alveus_asset_tracking::ResourceHandles>();
    app.insert_resource(SavePath(test_save_path.to_string()));

    app.add_plugins(StatsPlugin);

    // Initialize state transitions
    app.update();

    begin_play_in_world(app.world_mut());

    // Update app: should transition to Gameplay, load invalid save, and crash
    app.update();

    // Cleanup
    let _ = std::fs::remove_file(test_save_path);
}

#[test]
fn test_save_exclude_and_hydration() {
    let test_save_path = "exclude_hydration_test_save.ron";
    let _ = std::fs::remove_file(test_save_path);

    let mut app = App::new();
    app.add_plugins(StatesPlugin);
    app.add_plugins(alveus_app::plugin);
    app.add_plugins(MinimalPlugins);
    app.add_plugins(AssetPlugin::default());
    app.init_resource::<ButtonInput<KeyCode>>();
    app.init_resource::<alveus_asset_tracking::ResourceHandles>();
    app.insert_resource(SavePath(test_save_path.to_string()));

    app.add_plugins(StatsPlugin);

    // Initialize state transitions (this spawns default stats because no save exists)
    app.update();

    begin_play_in_world(app.world_mut());
    app.update();

    // Verify screen is gameplay
    assert_eq!(
        *app.world().resource::<State<Screen>>().get(),
        Screen::Gameplay
    );

    // Verify that the default entities have static components (e.g. AnimalName, EnclosureName)
    let polly_entity = app
        .world_mut()
        .query_filtered::<Entity, With<alveus_stats::AnimalId>>()
        .iter(app.world())
        .find(|&e| {
            *app.world().get::<alveus_stats::AnimalId>(e).unwrap() == alveus_stats::AnimalId::Polly
        })
        .expect("Polly should exist");

    assert!(
        app.world()
            .get::<alveus_stats::AnimalName>(polly_entity)
            .is_some()
    );
    assert!(
        app.world()
            .get::<alveus_stats::AnimalEnclosure>(polly_entity)
            .is_some()
    );

    // Modify Polly's hunger stats so we can verify it gets saved and loaded
    {
        let mut stats = app
            .world_mut()
            .get_mut::<alveus_stats::AnimalStats>(polly_entity)
            .unwrap();
        stats.hunger = Stat(450);
    }

    // Now let's trigger a save.
    let now_unix = std::time::SystemTime::now()
        .duration_since(std::time::SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    // Spawn the SaveTimestamp entity if not exists
    app.world_mut().spawn((
        Name::new("Save Timestamp"),
        alveus_stats::SaveTimestamp { value: now_unix },
    ));

    // Trigger SaveWorld with allowlist
    let mut save = SaveWorld::default_into_file(test_save_path);
    save.components = bevy::world_serialization::WorldFilter::deny_all()
        .allow::<alveus_stats::SaveTimestamp>()
        .allow::<alveus_stats::AnimalId>()
        .allow::<alveus_stats::AnimalStats>()
        .allow::<alveus_stats::EnclosureId>()
        .allow::<alveus_stats::EnclosureStats>();
    app.world_mut().trigger_save(save);

    // Run update to make sure save completes (commands are applied)
    app.update();

    // Now let's read the save file content and verify that it does NOT contain "AnimalName" or "AnimalEnclosure" or "Name"!
    let save_content = std::fs::read_to_string(test_save_path).expect("Save file not written");
    assert!(
        !save_content.contains("AnimalName"),
        "Save file should not contain AnimalName"
    );
    assert!(
        !save_content.contains("AnimalEnclosure"),
        "Save file should not contain AnimalEnclosure"
    );
    assert!(
        !save_content.contains("Name"),
        "Save file should not contain Name component"
    );
    assert!(
        save_content.contains("AnimalStats"),
        "Save file should contain AnimalStats"
    );
    assert!(
        save_content.contains("SaveTimestamp"),
        "Save file should contain SaveTimestamp"
    );

    // Clean up current stats entities to simulate starting the game fresh with the save file
    let mut entities_to_despawn = Vec::new();
    for entity in app
        .world_mut()
        .query_filtered::<Entity, Or<(
            With<alveus_stats::SaveTimestamp>,
            With<alveus_stats::AnimalId>,
            With<alveus_stats::EnclosureId>,
        )>>()
        .iter(app.world())
    {
        entities_to_despawn.push(entity);
    }
    for entity in entities_to_despawn {
        app.world_mut().despawn(entity);
    }

    // Insert the SavePath resource again (or keep it) and transition to Gameplay
    app.insert_resource(NextState::Pending(Screen::Title));
    app.update();
    app.insert_resource(NextState::Pending(Screen::Gameplay));
    app.update();
    app.update(); // runs hydration and updates

    // Now, verify that the loaded entity got hydrated with static components!
    let loaded_polly_entity = app
        .world_mut()
        .query_filtered::<Entity, With<alveus_stats::AnimalId>>()
        .iter(app.world())
        .find(|&e| {
            *app.world().get::<alveus_stats::AnimalId>(e).unwrap() == alveus_stats::AnimalId::Polly
        })
        .expect("Loaded Polly should exist");

    // Check stats are loaded correctly (hunger is 450)
    let stats = app
        .world()
        .get::<alveus_stats::AnimalStats>(loaded_polly_entity)
        .expect("Stats missing");
    assert_eq!(stats.hunger, Stat(450), "Hunger stat should be restored");

    // Check static components are hydrated!
    let name = app
        .world()
        .get::<alveus_stats::AnimalName>(loaded_polly_entity)
        .expect("Name not hydrated");
    assert_eq!(name.0, "Polly", "Name should be hydrated");

    let enc = app
        .world()
        .get::<alveus_stats::AnimalEnclosure>(loaded_polly_entity)
        .expect("Enclosure not hydrated");
    assert_eq!(
        enc.0,
        alveus_stats::EnclosureId::NutritionHousePlaypen,
        "Enclosure should be hydrated"
    );

    // Cleanup
    let _ = std::fs::remove_file(test_save_path);
}

#[test]
fn test_play_without_save() {
    let mut app = App::new();
    app.add_plugins(StatesPlugin);
    app.add_plugins(alveus_app::plugin);
    app.add_plugins(MinimalPlugins);
    app.add_plugins(AssetPlugin::default());
    app.init_resource::<ButtonInput<KeyCode>>();
    app.init_resource::<alveus_asset_tracking::ResourceHandles>();
    app.insert_resource(SavePath("nonexistent_play_without_save.ron".to_string()));

    app.add_plugins(StatsPlugin);

    // Initialize state transitions
    app.update();

    begin_play_in_world(app.world_mut());

    // Update app: should transition to Gameplay screen and initialize stats without crashing
    app.update();

    // Assert that the screen state is gameplay and animal entities were spawned
    assert_eq!(
        *app.world().resource::<State<Screen>>().get(),
        Screen::Gameplay
    );

    let animal_count = app
        .world_mut()
        .query::<&alveus_stats::AnimalId>()
        .iter(app.world())
        .count();
    assert_eq!(
        animal_count, 5,
        "Should initialize default 5 animals when save does not exist"
    );
}
