use alveus_idle_cli::cleaning::{
    cleanliness_after_threshold_decay, cleanliness_decay_with_poops, target_poop_count,
    try_dump_poop, try_pickup_poop, PoopDumpedEvent, PoopPickedUpEvent, PoopWheelbarrow,
    poop_config_for, WHEELBARROW_CAPACITY, CleaningPlugin,
};
use bevy::prelude::{Assets, ColorMaterial, Entity, Mesh, With};
use alveus_idle_cli::collision::{CollisionMasks, DynamicObstacleTiles};
use alveus_idle_cli::components::TilePosition;
use alveus_idle_cli::stats::{
    EnclosureId, EnclosureStat, EnclosureStats, StatTarget, WorsenStatEvent,
};
use moonshine_save::prelude::SaveWorld;
use moonshine_save::load::TriggerLoad;
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
    assert_eq!(
        try_dump_poop(&wheelbarrow).unwrap(),
        wheelbarrow.poops
    );
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
    let thresholds = &[800, 500, 200];
    assert_eq!(target_poop_count(1000, thresholds), 0);
    assert_eq!(target_poop_count(801, thresholds), 0);
    assert_eq!(target_poop_count(800, thresholds), 1);
    assert_eq!(target_poop_count(501, thresholds), 1);
    assert_eq!(target_poop_count(500, thresholds), 2);
    assert_eq!(target_poop_count(201, thresholds), 2);
    assert_eq!(target_poop_count(200, thresholds), 3);
    assert_eq!(target_poop_count(0, thresholds), 3);
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
        .find(|&e| {
            *app.world().get::<EnclosureId>(e).unwrap() == EnclosureId::PushPopEnclosure
        })
        .expect("Push Pop enclosure stats entity");

    app.world_mut().trigger(WorsenStatEvent {
        target: StatTarget::Enclosure {
            id: EnclosureId::PushPopEnclosure,
            stat: EnclosureStat::Cleanliness,
        },
        amount: 500,
    });
    assert_eq!(
        app.world().get::<EnclosureStats>(enc_entity).unwrap().cleanliness,
        500
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
    assert_eq!(stats.cleanliness, 850);

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
        .find(|&e| {
            *app.world().get::<EnclosureId>(e).unwrap() == EnclosureId::PushPopEnclosure
        })
        .expect("Push Pop enclosure stats entity");

    app.world_mut().trigger(WorsenStatEvent {
        target: StatTarget::Enclosure {
            id: EnclosureId::PushPopEnclosure,
            stat: EnclosureStat::Cleanliness,
        },
        amount: 999,
    });
    assert_eq!(
        app.world().get::<EnclosureStats>(enc_entity).unwrap().cleanliness,
        1
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
    assert_eq!(stats.cleanliness, 1);
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
        .find(|&e| {
            *app.world().get::<EnclosureId>(e).unwrap() == EnclosureId::PushPopEnclosure
        })
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

    let tiles = app
        .world()
        .get::<DynamicObstacleTiles>(enc_entity)
        .unwrap();
    assert!(!tiles.contains(tile));
    assert!(app.world().get_entity(poop_entity).is_err());

    common::cleanup_save(save_path);
}

#[test]
fn test_cleanliness_after_threshold_decay_24h_from_full() {
    let config = poop_config_for(EnclosureId::PushPopEnclosure);
    assert_eq!(
        cleanliness_after_threshold_decay(1000, 24.0, 30.0, config),
        0,
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
        wheelbarrow.poops = vec![
            EnclosureId::PushPopEnclosure,
            EnclosureId::PushPopEnclosure,
        ];
    }

    let mut save = SaveWorld::default_into_file(save_path);
    save.resources = bevy::world_serialization::WorldFilter::deny_all()
        .allow::<PoopWheelbarrow>();
    app.world_mut().trigger_save(save);
    app.update();

    let content = std::fs::read_to_string(save_path).expect("save written");
    assert!(content.contains("PoopWheelbarrow"));
    assert!(content.contains("PushPopEnclosure"));

    app.world_mut().resource_mut::<PoopWheelbarrow>().poops.clear();
    assert_eq!(app.world().resource::<PoopWheelbarrow>().count(), 0);

    app.world_mut().trigger_load(moonshine_save::prelude::LoadWorld::default_from_file(save_path));
    app.update();

    assert_eq!(app.world().resource::<PoopWheelbarrow>().count(), 2);

    common::cleanup_save(save_path);
}

#[test]
fn push_pop_poop_config_matches_design_intent() {
    let config = poop_config_for(EnclosureId::PushPopEnclosure);
    assert_eq!(config.spawn_thresholds, &[800, 500, 200]);
    assert_eq!(config.cleanliness_restore_per_poop, 350);
}
