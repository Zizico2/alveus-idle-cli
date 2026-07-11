use std::collections::HashSet;
use std::time::{SystemTime, UNIX_EPOCH};

use alveus_cleaning::{
    CleaningPlugin, PoopDumpedEvent, PoopPickedUpEvent, PoopWheelbarrow, WHEELBARROW_CAPACITY,
    cleanliness_after_threshold_decay, cleanliness_decay_with_poops, poop_config_for,
    target_poop_count, try_dump_poop, try_pickup_poop,
};
use alveus_collision::{CollisionMasks, DynamicObstacleTiles};
use alveus_components::TilePosition;
use alveus_stats::{
    EnclosureId, EnclosureStat, EnclosureStats, SaveTimestamp, StatTarget, WorsenStatEvent,
    advance_simulated_hours_world,
};
use alveus_types::{CleanStat, Stat};
use bevy::prelude::{Assets, ColorMaterial, Entity, Mesh, With};
use moonshine_save::load::TriggerLoad;
use moonshine_save::prelude::SaveWorld;
use moonshine_save::save::TriggerSave;

mod common;

#[test]
fn test_try_pickup_poop_fills_wheelbarrow() {
    let mut wheelbarrow = PoopWheelbarrow::default();
    assert!(try_pickup_poop(&mut wheelbarrow, EnclosureId::PushPopEnclosure).is_ok());
    assert_eq!(wheelbarrow.count(), 1);
    assert_eq!(wheelbarrow.poops[0], EnclosureId::PushPopEnclosure);
}

#[test]
fn test_try_pickup_poop_rejects_at_capacity() {
    let mut wheelbarrow = PoopWheelbarrow {
        poops: vec![EnclosureId::PushPopEnclosure; WHEELBARROW_CAPACITY as usize],
    };
    assert!(try_pickup_poop(&mut wheelbarrow, EnclosureId::PushPopEnclosure).is_err());
    assert_eq!(wheelbarrow.count(), WHEELBARROW_CAPACITY);
}

#[test]
fn test_try_dump_poop_requires_contents() {
    let wheelbarrow = PoopWheelbarrow::default();
    assert!(try_dump_poop(&wheelbarrow).is_err());
}

#[test]
fn test_try_dump_poop_returns_poops() {
    let wheelbarrow = PoopWheelbarrow {
        poops: vec![
            EnclosureId::PushPopEnclosure,
            EnclosureId::PushPopEnclosure,
            EnclosureId::PushPopEnclosure,
        ],
    };
    assert_eq!(try_dump_poop(&wheelbarrow).unwrap(), wheelbarrow.poops);
}

#[test]
fn test_cleanliness_decay_with_poops() {
    let base = 30.0;
    assert_eq!(
        cleanliness_decay_with_poops(base, EnclosureId::PushPopEnclosure, 0),
        30.0
    );
    assert_eq!(
        cleanliness_decay_with_poops(base, EnclosureId::PushPopEnclosure, 3),
        90.0
    );
}

#[test]
fn test_target_poop_count_from_cleanliness_thresholds() {
    let thresholds = &[Stat(800), Stat(500), Stat(200)];
    assert_eq!(target_poop_count(Stat(1000), thresholds), 0);
    assert_eq!(target_poop_count(Stat(801), thresholds), 0);
    assert_eq!(target_poop_count(Stat(800), thresholds), 1);
    assert_eq!(target_poop_count(Stat(501), thresholds), 1);
    assert_eq!(target_poop_count(Stat(500), thresholds), 2);
    assert_eq!(target_poop_count(Stat(201), thresholds), 2);
    assert_eq!(target_poop_count(Stat(200), thresholds), 3);
    assert_eq!(target_poop_count(Stat(0), thresholds), 3);
}

#[test]
fn test_poop_pickup_restores_cleanliness() {
    let save_path = "test_clean_pickup_restore.ron";
    common::cleanup_save(save_path);

    let mut app = common::minimal_stats_app(save_path);
    app.init_resource::<Assets<Mesh>>();
    app.init_resource::<Assets<ColorMaterial>>();
    app.init_resource::<CollisionMasks>();
    app.add_plugins(CleaningPlugin);

    let enc_entity = app
        .world_mut()
        .query_filtered::<Entity, With<EnclosureId>>()
        .iter(app.world())
        .find(|&e| *app.world().get::<EnclosureId>(e).unwrap() == EnclosureId::PushPopEnclosure)
        .expect("Push Pop enclosure stats entity");

    app.world_mut().trigger(WorsenStatEvent {
        target: StatTarget::Enclosure {
            id: EnclosureId::PushPopEnclosure,
            stat: EnclosureStat::Cleanliness,
        },
        amount: Stat(500),
    });
    assert_eq!(
        app.world()
            .get::<EnclosureStats>(enc_entity)
            .unwrap()
            .cleanliness,
        Stat(500)
    );

    let tile = TilePosition { x: 7, y: 5 };
    {
        let mut tiles = app
            .world_mut()
            .get_mut::<DynamicObstacleTiles>(enc_entity)
            .unwrap();
        tiles.insert(tile);
    }

    let poop_entity = app.world_mut().spawn_empty().id();
    app.insert_resource(PoopWheelbarrow {
        poops: vec![EnclosureId::PushPopEnclosure],
    });
    app.world_mut().trigger(PoopPickedUpEvent {
        entity: poop_entity,
        enclosure_id: EnclosureId::PushPopEnclosure,
        tile,
    });
    app.update();

    let stats = app.world().get::<EnclosureStats>(enc_entity).unwrap();
    assert_eq!(stats.cleanliness, Stat(850));

    common::cleanup_save(save_path);
}

#[test]
fn test_poop_dump_does_not_restore_cleanliness() {
    let save_path = "test_clean_dump.ron";
    common::cleanup_save(save_path);

    let mut app = common::minimal_stats_app(save_path);
    app.init_resource::<Assets<Mesh>>();
    app.init_resource::<Assets<ColorMaterial>>();
    app.init_resource::<CollisionMasks>();
    app.add_plugins(CleaningPlugin);

    let enc_entity = app
        .world_mut()
        .query_filtered::<Entity, With<EnclosureId>>()
        .iter(app.world())
        .find(|&e| *app.world().get::<EnclosureId>(e).unwrap() == EnclosureId::PushPopEnclosure)
        .expect("Push Pop enclosure stats entity");

    app.world_mut().trigger(WorsenStatEvent {
        target: StatTarget::Enclosure {
            id: EnclosureId::PushPopEnclosure,
            stat: EnclosureStat::Cleanliness,
        },
        amount: Stat(999),
    });
    assert_eq!(
        app.world()
            .get::<EnclosureStats>(enc_entity)
            .unwrap()
            .cleanliness,
        Stat(1)
    );

    app.world_mut().trigger(PoopDumpedEvent {
        poops: vec![
            EnclosureId::PushPopEnclosure,
            EnclosureId::PushPopEnclosure,
            EnclosureId::PushPopEnclosure,
        ],
    });
    app.update();

    let stats = app.world().get::<EnclosureStats>(enc_entity).unwrap();
    assert_eq!(stats.cleanliness, Stat(1));
    assert_eq!(app.world().resource::<PoopWheelbarrow>().count(), 0);

    common::cleanup_save(save_path);
}

#[test]
fn test_poop_pickup_removes_tile() {
    let save_path = "test_clean_pickup.ron";
    common::cleanup_save(save_path);

    let mut app = common::minimal_stats_app(save_path);
    app.init_resource::<Assets<Mesh>>();
    app.init_resource::<Assets<ColorMaterial>>();
    app.init_resource::<CollisionMasks>();
    app.add_plugins(CleaningPlugin);

    let enc_entity = app
        .world_mut()
        .query_filtered::<Entity, With<EnclosureId>>()
        .iter(app.world())
        .find(|&e| *app.world().get::<EnclosureId>(e).unwrap() == EnclosureId::PushPopEnclosure)
        .expect("Push Pop enclosure stats entity");

    let tile = TilePosition { x: 7, y: 5 };
    {
        let mut tiles = app
            .world_mut()
            .get_mut::<DynamicObstacleTiles>(enc_entity)
            .unwrap();
        tiles.insert(tile);
    }

    let poop_entity = app.world_mut().spawn_empty().id();
    app.insert_resource(PoopWheelbarrow {
        poops: vec![EnclosureId::PushPopEnclosure],
    });
    app.world_mut().trigger(PoopPickedUpEvent {
        entity: poop_entity,
        enclosure_id: EnclosureId::PushPopEnclosure,
        tile,
    });
    app.update();

    let tiles = app.world().get::<DynamicObstacleTiles>(enc_entity).unwrap();
    assert!(!tiles.contains(tile));
    assert!(app.world().get_entity(poop_entity).is_err());

    common::cleanup_save(save_path);
}

#[test]
fn test_cleanliness_after_threshold_decay_24h_from_full() {
    let config = poop_config_for(EnclosureId::PushPopEnclosure).expect("Push Pop has poop config");
    assert_eq!(
        cleanliness_after_threshold_decay(Stat(1000), 24.0, 30.0, config),
        Stat(0),
        "24h segmented decay from 100% should reach 0%"
    );
}

#[test]
fn test_wheelbarrow_persists_in_save() {
    let save_path = "test_wheelbarrow_save.ron";
    common::cleanup_save(save_path);

    let mut app = common::minimal_stats_app(save_path);
    app.add_plugins(CleaningPlugin);
    {
        let mut wheelbarrow = app.world_mut().resource_mut::<PoopWheelbarrow>();
        wheelbarrow.poops = vec![EnclosureId::PushPopEnclosure, EnclosureId::PushPopEnclosure];
    }

    let mut save = SaveWorld::default_into_file(save_path);
    save.resources = bevy::world_serialization::WorldFilter::deny_all().allow::<PoopWheelbarrow>();
    app.world_mut().trigger_save(save);
    app.update();

    let content = std::fs::read_to_string(save_path).expect("save written");
    assert!(content.contains("PoopWheelbarrow"));
    assert!(content.contains("PushPopEnclosure"));

    app.world_mut()
        .resource_mut::<PoopWheelbarrow>()
        .poops
        .clear();
    assert_eq!(app.world().resource::<PoopWheelbarrow>().count(), 0);

    app.world_mut()
        .trigger_load(moonshine_save::prelude::LoadWorld::default_from_file(
            save_path,
        ));
    app.update();

    assert_eq!(app.world().resource::<PoopWheelbarrow>().count(), 2);

    common::cleanup_save(save_path);
}

#[test]
fn push_pop_poop_config_matches_configs() {
    let config = poop_config_for(EnclosureId::PushPopEnclosure).expect("Push Pop has poop config");
    assert_eq!(config.spawn_thresholds, &[Stat(800), Stat(500), Stat(200)]);
    assert_eq!(config.cleanliness_restore_per_poop, CleanStat(Stat(350)));
}

#[test]
fn nutrition_house_has_no_poop_config() {
    assert!(
        poop_config_for(EnclosureId::NutritionHousePlaypen).is_none(),
        "Polly clean is nest-sweep only; no pile-based poop config"
    );
}
#[test]
fn test_poop_count_accelerates_offline_decay() {
    let save_path = "test_clean_decay.ron";
    common::cleanup_save(save_path);

    let mut app = common::minimal_stats_app(save_path);

    let enc_entity = app
        .world_mut()
        .query_filtered::<Entity, With<EnclosureId>>()
        .iter(app.world())
        .find(|&e| *app.world().get::<EnclosureId>(e).unwrap() == EnclosureId::PushPopEnclosure)
        .expect("Push Pop enclosure stats entity");

    {
        let mut stats = app
            .world_mut()
            .get_mut::<EnclosureStats>(enc_entity)
            .unwrap();
        // At 20% with three floor poops (threshold band), decay runs at 30 + 3*20 = 90/h.
        stats.cleanliness = Stat(200);
        let mut tiles = app
            .world_mut()
            .get_mut::<DynamicObstacleTiles>(enc_entity)
            .unwrap();
        tiles.insert(TilePosition { x: 6, y: 4 });
        tiles.insert(TilePosition { x: 7, y: 4 });
        tiles.insert(TilePosition { x: 8, y: 4 });
    }

    advance_simulated_hours_world(app.world_mut(), 10.0);
    app.update();

    let stats = app.world().get::<EnclosureStats>(enc_entity).unwrap();
    // 10h * 90/h = 900 decay from 200 -> 0
    assert_eq!(stats.cleanliness, Stat(0));

    common::cleanup_save(save_path);
}

#[test]
fn test_threshold_poop_spawn_on_cleanliness() {
    let save_path = "test_threshold_poops.ron";
    common::cleanup_save(save_path);

    let mut app = common::minimal_stats_app(save_path);
    app.init_resource::<CollisionMasks>();
    app.add_plugins(CleaningPlugin);

    {
        let mut masks = app.world_mut().resource_mut::<CollisionMasks>();
        masks.set_static_mask(alveus_collision::CollisionMapKey::Overview, HashSet::new());
        masks.set_static_mask(
            alveus_collision::CollisionMapKey::Enclosure(EnclosureId::NutritionHousePlaypen),
            HashSet::new(),
        );
        masks.set_static_mask(
            alveus_collision::CollisionMapKey::Enclosure(EnclosureId::PushPopEnclosure),
            HashSet::new(),
        );
    }

    let enc_entity = app
        .world_mut()
        .query_filtered::<Entity, With<EnclosureId>>()
        .iter(app.world())
        .find(|&e| *app.world().get::<EnclosureId>(e).unwrap() == EnclosureId::PushPopEnclosure)
        .expect("Push Pop enclosure stats entity");

    {
        let mut stats = app
            .world_mut()
            .get_mut::<EnclosureStats>(enc_entity)
            .unwrap();
        stats.cleanliness = Stat(800);
    }
    app.update();

    let tiles = app.world().get::<DynamicObstacleTiles>(enc_entity).unwrap();
    assert_eq!(tiles.0.len(), 1, "one poop at 80% cleanliness");

    {
        let mut stats = app
            .world_mut()
            .get_mut::<EnclosureStats>(enc_entity)
            .unwrap();
        stats.cleanliness = Stat(500);
    }
    app.update();

    let tiles = app.world().get::<DynamicObstacleTiles>(enc_entity).unwrap();
    assert_eq!(tiles.0.len(), 2, "two poops at 50% cleanliness");

    {
        let mut stats = app
            .world_mut()
            .get_mut::<EnclosureStats>(enc_entity)
            .unwrap();
        stats.cleanliness = Stat(200);
    }
    app.update();

    let tiles = app.world().get::<DynamicObstacleTiles>(enc_entity).unwrap();
    assert_eq!(tiles.0.len(), 3, "three poops at 20% cleanliness");

    common::cleanup_save(save_path);
}

#[test]
fn test_decay_spawns_poops_when_crossing_thresholds() {
    let save_path = "test_decay_threshold_poops.ron";
    common::cleanup_save(save_path);

    let mut app = common::minimal_stats_app(save_path);
    app.init_resource::<CollisionMasks>();
    app.add_plugins(CleaningPlugin);

    {
        let mut masks = app.world_mut().resource_mut::<CollisionMasks>();
        masks.set_static_mask(alveus_collision::CollisionMapKey::Overview, HashSet::new());
        masks.set_static_mask(
            alveus_collision::CollisionMapKey::Enclosure(EnclosureId::NutritionHousePlaypen),
            HashSet::new(),
        );
        masks.set_static_mask(
            alveus_collision::CollisionMapKey::Enclosure(EnclosureId::PushPopEnclosure),
            HashSet::new(),
        );
    }
    app.update();

    let enc_entity = app
        .world_mut()
        .query_filtered::<Entity, With<EnclosureId>>()
        .iter(app.world())
        .find(|&e| *app.world().get::<EnclosureId>(e).unwrap() == EnclosureId::PushPopEnclosure)
        .expect("Push Pop enclosure stats entity");

    advance_simulated_hours_world(app.world_mut(), 7.0);
    app.update();

    let stats = app.world().get::<EnclosureStats>(enc_entity).unwrap();
    assert!(
        stats.cleanliness <= Stat(800),
        "decay should reach first threshold"
    );
    let tiles = app.world().get::<DynamicObstacleTiles>(enc_entity).unwrap();
    assert_eq!(
        tiles.0.len(),
        1,
        "first poop spawns after decay crosses 80%"
    );

    common::cleanup_save(save_path);
}

#[test]
fn test_offline_decay_from_full_spawns_three_poops() {
    let save_path = "test_offline_threshold_poops.ron";
    common::cleanup_save(save_path);

    let mut app = common::minimal_stats_app(save_path);
    app.init_resource::<CollisionMasks>();
    app.add_plugins(CleaningPlugin);

    {
        let mut masks = app.world_mut().resource_mut::<CollisionMasks>();
        masks.set_static_mask(alveus_collision::CollisionMapKey::Overview, HashSet::new());
        masks.set_static_mask(
            alveus_collision::CollisionMapKey::Enclosure(EnclosureId::NutritionHousePlaypen),
            HashSet::new(),
        );
        masks.set_static_mask(
            alveus_collision::CollisionMapKey::Enclosure(EnclosureId::PushPopEnclosure),
            HashSet::new(),
        );
    }

    let enc_entity = app
        .world_mut()
        .query_filtered::<Entity, With<EnclosureId>>()
        .iter(app.world())
        .find(|&e| *app.world().get::<EnclosureId>(e).unwrap() == EnclosureId::PushPopEnclosure)
        .expect("Push Pop enclosure stats entity");

    {
        let mut stats = app
            .world_mut()
            .get_mut::<EnclosureStats>(enc_entity)
            .unwrap();
        stats.cleanliness = Stat(1000);
        app.world_mut()
            .get_mut::<DynamicObstacleTiles>(enc_entity)
            .unwrap()
            .0
            .clear();
    }

    // 40h offline at 30/h passive decay drains 1000 -> 0 with no floor poops yet.
    let hours_offline = 40.0;
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock before unix epoch")
        .as_secs();
    app.world_mut().spawn((SaveTimestamp {
        value: now.saturating_sub((hours_offline * 3600.0) as u64),
    },));

    app.update();

    let stats = app.world().get::<EnclosureStats>(enc_entity).unwrap();
    assert_eq!(
        stats.cleanliness,
        Stat(0),
        "offline catch-up should drain enclosure cleanliness to 0"
    );

    let tiles = app.world().get::<DynamicObstacleTiles>(enc_entity).unwrap();
    assert_eq!(
        tiles.0.len(),
        3,
        "three poops should spawn when offline decay reaches 0% cleanliness"
    );

    common::cleanup_save(save_path);
}

#[test]
fn test_offline_decay_accelerates_with_spawned_poops() {
    let save_path = "test_offline_poop_accel.ron";
    common::cleanup_save(save_path);

    let mut app = common::minimal_stats_app(save_path);
    app.init_resource::<CollisionMasks>();
    app.add_plugins(CleaningPlugin);

    {
        let mut masks = app.world_mut().resource_mut::<CollisionMasks>();
        masks.set_static_mask(alveus_collision::CollisionMapKey::Overview, HashSet::new());
        masks.set_static_mask(
            alveus_collision::CollisionMapKey::Enclosure(EnclosureId::NutritionHousePlaypen),
            HashSet::new(),
        );
        masks.set_static_mask(
            alveus_collision::CollisionMapKey::Enclosure(EnclosureId::PushPopEnclosure),
            HashSet::new(),
        );
    }

    let enc_entity = app
        .world_mut()
        .query_filtered::<Entity, With<EnclosureId>>()
        .iter(app.world())
        .find(|&e| *app.world().get::<EnclosureId>(e).unwrap() == EnclosureId::PushPopEnclosure)
        .expect("Push Pop enclosure stats entity");

    {
        let mut stats = app
            .world_mut()
            .get_mut::<EnclosureStats>(enc_entity)
            .unwrap();
        stats.cleanliness = Stat(1000);
        app.world_mut()
            .get_mut::<DynamicObstacleTiles>(enc_entity)
            .unwrap()
            .0
            .clear();
    }

    // 24h flat decay: 720 drain -> 280 (28%) -> 2 poops.
    // Segmented (poops spawn at thresholds): reaches 0% -> 3 poops.
    let hours_offline = 24.0;
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock before unix epoch")
        .as_secs();
    app.world_mut().spawn((SaveTimestamp {
        value: now.saturating_sub((hours_offline * 3600.0) as u64),
    },));

    app.update();

    let stats = app.world().get::<EnclosureStats>(enc_entity).unwrap();
    assert_eq!(
        stats.cleanliness,
        Stat(0),
        "24h offline with threshold poop acceleration should drain to 0, not stop at 280"
    );

    let tiles = app.world().get::<DynamicObstacleTiles>(enc_entity).unwrap();
    assert_eq!(
        tiles.0.len(),
        3,
        "24h offline should cross all three thresholds and spawn three poops"
    );

    common::cleanup_save(save_path);
}
