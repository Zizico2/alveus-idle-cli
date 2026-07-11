use alveus_collision::{
    CollisionMapKey, REQUIRED_COLLISION_KEYS, any_required_collision_map_failed,
    build_mask_for_asset, collision_ready, required_collision_handles,
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
fn all_required_collision_maps_load_and_build_masks() {
    use alveus_collision::{CollisionMasks, InteriorAssets, LevelAssets, build_all_collision_masks};
    use alveus_types::EnclosureId;

    let mut app = common::headless_tiled_test_app();
    app.init_resource::<CollisionMasks>();

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

    // Warm the paths after retaining the same strong handles production keeps.
    for key in REQUIRED_COLLISION_KEYS {
        let _ = common::load_tiled_map(&mut app, key.asset_path());
    }

    app.world_mut()
        .resource_scope(|world, mut masks: Mut<CollisionMasks>| {
            let map_assets = world.resource::<Assets<TiledMapAsset>>();
            let level_assets = world.resource::<LevelAssets>();
            let interior_assets = world.resource::<InteriorAssets>();
            build_all_collision_masks(&mut masks, map_assets, level_assets, interior_assets);
        });

    let failed = {
        let asset_server = app.world().resource::<AssetServer>();
        let level = app.world().resource::<LevelAssets>();
        let interior = app.world().resource::<InteriorAssets>();
        let handles = required_collision_handles(level, interior);
        any_required_collision_map_failed(asset_server, &handles)
    };
    assert!(!failed, "shipped maps must load");

    let masks = app.world().resource::<CollisionMasks>();
    assert!(
        collision_ready(masks),
        "all REQUIRED_COLLISION_KEYS must have masks"
    );
    for key in REQUIRED_COLLISION_KEYS {
        assert!(masks.contains(*key), "missing mask for {key:?}");
    }
}
