use alveus_app::Screen;
use alveus_stats::{
    AnimalEnclosure, AnimalId, AnimalName, AnimalStat, AnimalStats, EnclosureId, EnclosureName,
    EnclosureStat, EnclosureStats, ImproveStatEvent, SanctuaryUpkeep, SavePath, StatTarget,
    StatsPlugin, WorsenStatEvent, tick_decay_system,
};
use alveus_types::Stat;
use bevy::ecs::system::RunSystemOnce;
use bevy::prelude::*;
use bevy::state::app::StatesPlugin;

#[test]
fn test_stats_initialization() {
    let save_path = "nonexistent_save_init.ron";
    let _ = std::fs::remove_file(save_path);
    let mut app = App::new();
    app.add_plugins(StatesPlugin);
    app.init_state::<Screen>();
    app.add_plugins(MinimalPlugins);
    app.init_resource::<ButtonInput<KeyCode>>();
    app.insert_resource(SavePath(save_path.to_string()));
    app.add_plugins(StatsPlugin);

    // Enter Gameplay screen to trigger init_stats_system
    app.insert_resource(NextState::Pending(Screen::Gameplay));
    app.update(); // runs state transitions and system updates

    // Verify animal stats entities are spawned
    let animals: Vec<(AnimalId, String, AnimalStats, EnclosureId)> = {
        let mut animal_query =
            app.world_mut()
                .query::<(&AnimalId, &AnimalName, &AnimalStats, &AnimalEnclosure)>();
        animal_query
            .iter(app.world())
            .map(|(id, name, stats, enc)| (*id, name.0.clone(), stats.clone(), enc.0))
            .collect()
    };
    assert_eq!(animals.len(), 5, "Should spawn 5 animals");

    // Verify enclosure stats entities are spawned
    let enclosures: Vec<(EnclosureId, String, EnclosureStats)> = {
        let mut enclosure_query = app
            .world_mut()
            .query::<(&EnclosureId, &EnclosureName, &EnclosureStats)>();
        enclosure_query
            .iter(app.world())
            .map(|(id, name, stats)| (*id, name.0.clone(), stats.clone()))
            .collect()
    };
    assert_eq!(
        enclosures.len(),
        4,
        "Should spawn 4 enclosures (Playpen, Push Pop, Pasture, Reptile)"
    );

    // Find Georgie and Siren and verify they share Reptile Enclosure
    let mut push_pop_enc = None;
    let mut georgie_enc = None;
    let mut siren_enc = None;
    for (id, _name, _stats, enc) in &animals {
        if *id == AnimalId::PushPop {
            push_pop_enc = Some(*enc);
        } else if *id == AnimalId::Georgie {
            georgie_enc = Some(*enc);
        } else if *id == AnimalId::Siren {
            siren_enc = Some(*enc);
        }
    }
    assert_eq!(push_pop_enc, Some(EnclosureId::PushPopEnclosure));
    assert_eq!(georgie_enc, Some(EnclosureId::ReptileEnclosure));
    assert_eq!(siren_enc, Some(EnclosureId::ReptileEnclosure));
    let _ = std::fs::remove_file(save_path);
}

#[test]
fn test_stat_observers_and_clamping() {
    let save_path = "nonexistent_save_obs.ron";
    let _ = std::fs::remove_file(save_path);
    let mut app = App::new();
    app.add_plugins(StatesPlugin);
    app.init_state::<Screen>();
    app.add_plugins(MinimalPlugins);
    app.init_resource::<ButtonInput<KeyCode>>();
    app.insert_resource(SavePath(save_path.to_string()));
    app.add_plugins(StatsPlugin);

    // Transition to Gameplay to spawn entities
    app.insert_resource(NextState::Pending(Screen::Gameplay));
    app.update();

    // Verify initial state
    let polly_entity = app
        .world_mut()
        .query_filtered::<Entity, With<AnimalId>>()
        .iter(app.world())
        .find(|&e| *app.world().get::<AnimalId>(e).unwrap() == AnimalId::Polly)
        .expect("Polly should exist");

    let initial_stats = app.world().get::<AnimalStats>(polly_entity).unwrap();
    assert_eq!(initial_stats.hunger, Stat(1000));

    // Trigger worsening
    app.world_mut().trigger(WorsenStatEvent {
        target: StatTarget::Animal {
            id: AnimalId::Polly,
            stat: AnimalStat::Hunger,
        },
        amount: Stat(300),
    });
    // Observers trigger immediately on world trigger
    let stats = app.world().get::<AnimalStats>(polly_entity).unwrap();
    assert_eq!(stats.hunger, Stat(700));

    // Trigger improve
    app.world_mut().trigger(ImproveStatEvent {
        target: StatTarget::Animal {
            id: AnimalId::Polly,
            stat: AnimalStat::Hunger,
        },
        amount: Stat(200),
    });
    let stats = app.world().get::<AnimalStats>(polly_entity).unwrap();
    assert_eq!(stats.hunger, Stat(900));

    // Trigger improve past max to test clamping
    app.world_mut().trigger(ImproveStatEvent {
        target: StatTarget::Animal {
            id: AnimalId::Polly,
            stat: AnimalStat::Hunger,
        },
        amount: Stat(500),
    });
    let stats = app.world().get::<AnimalStats>(polly_entity).unwrap();
    assert_eq!(stats.hunger, Stat(1000));

    // Trigger worsening below 0 to test clamping
    app.world_mut().trigger(WorsenStatEvent {
        target: StatTarget::Animal {
            id: AnimalId::Polly,
            stat: AnimalStat::Hunger,
        },
        amount: Stat(1200),
    });
    let stats = app.world().get::<AnimalStats>(polly_entity).unwrap();
    assert_eq!(stats.hunger, Stat(0));
    let _ = std::fs::remove_file(save_path);
}

#[test]
fn test_shared_enclosure_cleanliness() {
    let save_path = "nonexistent_save_enc.ron";
    let _ = std::fs::remove_file(save_path);
    let mut app = App::new();
    app.add_plugins(StatesPlugin);
    app.init_state::<Screen>();
    app.add_plugins(MinimalPlugins);
    app.init_resource::<ButtonInput<KeyCode>>();
    app.insert_resource(SavePath(save_path.to_string()));
    app.add_plugins(StatsPlugin);

    // Transition to Gameplay to spawn entities
    app.insert_resource(NextState::Pending(Screen::Gameplay));
    app.update();

    let reptile_entity = app
        .world_mut()
        .query_filtered::<Entity, With<EnclosureId>>()
        .iter(app.world())
        .find(|&e| *app.world().get::<EnclosureId>(e).unwrap() == EnclosureId::ReptileEnclosure)
        .expect("Reptile Enclosure should exist");

    // Initial cleanliness is 1000
    let enc_stats = app.world().get::<EnclosureStats>(reptile_entity).unwrap();
    assert_eq!(enc_stats.cleanliness, Stat(1000));

    // Worsen Georgie's cleanliness (targets animal, observer resolves to reptile_enclosure)
    app.world_mut().trigger(WorsenStatEvent {
        target: StatTarget::Animal {
            id: AnimalId::Georgie,
            stat: AnimalStat::Cleanliness,
        },
        amount: Stat(400),
    });

    // Verify enclosure cleanliness drops
    let enc_stats = app.world().get::<EnclosureStats>(reptile_entity).unwrap();
    assert_eq!(enc_stats.cleanliness, Stat(600));

    // Improve Siren's cleanliness (should improve the shared reptile enclosure!)
    app.world_mut().trigger(ImproveStatEvent {
        target: StatTarget::Animal {
            id: AnimalId::Siren,
            stat: AnimalStat::Cleanliness,
        },
        amount: Stat(250),
    });

    // Verify enclosure cleanliness increases
    let enc_stats = app.world().get::<EnclosureStats>(reptile_entity).unwrap();
    assert_eq!(enc_stats.cleanliness, Stat(850));
    let _ = std::fs::remove_file(save_path);
}

#[test]
fn test_upkeep_calculation() {
    let save_path = "nonexistent_save_upkeep.ron";
    let _ = std::fs::remove_file(save_path);
    let mut app = App::new();
    app.add_plugins(StatesPlugin);
    app.init_state::<Screen>();
    app.add_plugins(MinimalPlugins);
    app.init_resource::<ButtonInput<KeyCode>>();
    app.insert_resource(SavePath(save_path.to_string()));
    app.add_plugins(StatsPlugin);

    // Transition to Gameplay to spawn entities
    app.insert_resource(NextState::Pending(Screen::Gameplay));
    app.update();

    // Worsen stats of specific animals/enclosures to test mean updates
    app.world_mut().trigger(WorsenStatEvent {
        target: StatTarget::Animal {
            id: AnimalId::Polly,
            stat: AnimalStat::Hunger,
        },
        amount: Stat(400), // Polly hunger becomes 600, others stay 1000 -> mean hunger = (600+1000*4)/5000 = 0.92
    });

    app.world_mut().trigger(WorsenStatEvent {
        target: StatTarget::Enclosure {
            id: EnclosureId::ReptileEnclosure,
            stat: EnclosureStat::Cleanliness,
        },
        amount: Stat(600), // reptile cleanliness becomes 400, other 3 enclosures stay 1000 -> mean = 3400/4000 = 0.85
    });

    // Run system updates to calculate upkeep
    app.update();

    let upkeep = app.world().resource::<SanctuaryUpkeep>();
    assert!((upkeep.mean_hunger - 0.92).abs() < 0.001);
    assert!((upkeep.mean_cleanliness - 0.85).abs() < 0.001);
    assert!((upkeep.mean_happiness - 1.0).abs() < 0.001);
    assert!((upkeep.score - 0.923333).abs() < 0.001);
    let _ = std::fs::remove_file(save_path);
}

#[test]
fn test_decay_rate() {
    let save_path = "nonexistent_save_decay.ron";
    let _ = std::fs::remove_file(save_path);
    let mut app = App::new();
    app.add_plugins(StatesPlugin);
    app.init_state::<Screen>();
    app.add_plugins(MinimalPlugins);
    app.init_resource::<ButtonInput<KeyCode>>();
    app.insert_resource(SavePath(save_path.to_string()));
    app.add_plugins(StatsPlugin);

    // Transition to Gameplay to spawn entities
    app.insert_resource(NextState::Pending(Screen::Gameplay));
    app.update();

    // Verify initial Polly entity stats
    let polly_entity = app
        .world_mut()
        .query_filtered::<Entity, With<AnimalId>>()
        .iter(app.world())
        .find(|&e| *app.world().get::<AnimalId>(e).unwrap() == AnimalId::Polly)
        .expect("Polly should exist");

    let initial_stats = app
        .world()
        .get::<AnimalStats>(polly_entity)
        .unwrap()
        .clone();
    assert_eq!(initial_stats.hunger, Stat(1000));
    assert_eq!(initial_stats.happiness, Stat(1000));

    // Set custom Time resource with delta of 2 hours
    let mut time = Time::<()>::default();
    time.advance_by(std::time::Duration::from_secs(7200)); // sets delta to 7200 seconds (2 hours)
    app.insert_resource(time);

    // Run the tick_decay_system once directly on the world
    let _ = app.world_mut().run_system_once(tick_decay_system);

    // Get updated stats
    let updated_stats = app.world().get::<AnimalStats>(polly_entity).unwrap();
    // Polly's hunger decay rate is 0.04 per hour (40 units per hour out of 1000).
    // In 2 hours, hunger should decay by 80 units.
    // Polly's happiness decay rate is 0.05 per hour (50 units per hour out of 1000).
    // In 2 hours, happiness should decay by 100 units.
    assert_eq!(updated_stats.hunger, Stat(920));
    assert_eq!(updated_stats.happiness, Stat(900));
    let _ = std::fs::remove_file(save_path);
}
