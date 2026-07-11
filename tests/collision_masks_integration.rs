use alveus_collision::{
    CollisionLoadFailures, CollisionMapKey, REQUIRED_COLLISION_KEYS, build_mask_for_asset,
    collision_ready, record_failed_collision_map_loads,
};
use alveus_components::TilePosition;
use bevy::prelude::*;
use bevy_ecs_tiled::prelude::TiledMapAsset;

mod common;

#[test]
fn interior_collision_masks_from_tiled_assets() {
    let mut app = common::headless_tiled_test_app();

    let nutrition = common::load_tiled_map(&mut app, "maps/interiors/nutrition_house_interior.tmx");
    let push_pop =
        common::load_tiled_map(&mut app, "maps/interiors/push_pop_enclosure_interior.tmx");

    let assets = app.world().resource::<Assets<TiledMapAsset>>();

    let nutrition_asset = assets.get(&nutrition).expect("nutrition interior asset");
    let push_pop_asset = assets.get(&push_pop).expect("push pop interior asset");

    let nutrition_obstacles = build_mask_for_asset(nutrition_asset);
    assert!(
        !nutrition_obstacles.is_empty(),
        "nutrition house interior should have obstacle tiles"
    );

    let push_pop_obstacles = build_mask_for_asset(push_pop_asset);
    assert!(
        push_pop_obstacles.contains(&TilePosition { x: 8, y: 6 }),
        "feeding dish tile should be blocked"
    );
    assert!(
        push_pop_obstacles.contains(&TilePosition { x: 3, y: 9 }),
        "shelter tile should be blocked"
    );
    assert!(
        !push_pop_obstacles.contains(&TilePosition { x: 8, y: 4 }),
        "Push Pop default home tile should be walkable"
    );
}

#[test]
fn overview_compost_bin_blocked_in_collision_mask() {
    let mut app = common::headless_tiled_test_app();

    let overview = common::load_tiled_map(&mut app, "maps/overview/map.tmx");
    let assets = app.world().resource::<Assets<TiledMapAsset>>();
    let overview_asset = assets.get(&overview).expect("overview map asset");

    let obstacles = build_mask_for_asset(overview_asset);
    assert!(
        obstacles.contains(&TilePosition { x: 3, y: 0 }),
        "overview compost bin at (3,0) should be blocked"
    );
}

#[test]
fn all_required_collision_maps_load_without_failures() {
    use alveus_collision::{CollisionMasks, InteriorAssets, LevelAssets, build_all_collision_masks};
    use alveus_types::EnclosureId;

    let mut app = common::headless_tiled_test_app();
    app.init_resource::<CollisionMasks>();
    app.init_resource::<CollisionLoadFailures>();
    {
        let handles = alveus_collision::RequiredCollisionMapHandles::from_asset_server(
            app.world().resource::<AssetServer>(),
        );
        app.insert_resource(handles);
    }

    // Warm the same paths LevelAssets / InteriorAssets use.
    for key in REQUIRED_COLLISION_KEYS {
        let _ = common::load_tiled_map(&mut app, key.asset_path());
    }

    let (level, interior) = {
        let server = app.world().resource::<AssetServer>();
        (
            LevelAssets {
                map: server.load(CollisionMapKey::Overview.asset_path()),
            },
            InteriorAssets {
                nutrition_house: server.load(
                    CollisionMapKey::Enclosure(EnclosureId::NutritionHousePlaypen).asset_path(),
                ),
                push_pop_enclosure: server
                    .load(CollisionMapKey::Enclosure(EnclosureId::PushPopEnclosure).asset_path()),
            },
        )
    };
    app.insert_resource(level);
    app.insert_resource(interior);

    app.world_mut().resource_scope(|world, mut masks: Mut<CollisionMasks>| {
        let map_assets = world.resource::<Assets<TiledMapAsset>>();
        let level_assets = world.resource::<LevelAssets>();
        let interior_assets = world.resource::<InteriorAssets>();
        build_all_collision_masks(&mut masks, map_assets, level_assets, interior_assets);
    });

    app.world_mut()
        .resource_scope(|world, mut failures: Mut<CollisionLoadFailures>| {
            let asset_server = world.resource::<AssetServer>();
            let required = world.resource::<alveus_collision::RequiredCollisionMapHandles>();
            let level = world.get_resource::<LevelAssets>();
            let interior = world.get_resource::<InteriorAssets>();
            let handles =
                alveus_collision::required_collision_handles(required, level, interior);
            let mut gate = alveus_collision::CollisionReloadGate::default();
            record_failed_collision_map_loads(asset_server, &handles, &mut failures, &gate);
        });

    let masks = app.world().resource::<CollisionMasks>();
    let failures = app.world().resource::<CollisionLoadFailures>();
    assert!(
        failures.is_empty(),
        "shipped maps must not produce load failures: {failures:?}"
    );
    assert!(
        collision_ready(masks),
        "all REQUIRED_COLLISION_KEYS must have masks"
    );
    for key in REQUIRED_COLLISION_KEYS {
        assert!(masks.contains(*key), "missing mask for {key:?}");
    }
}
