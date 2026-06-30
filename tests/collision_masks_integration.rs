use alveus_idle_cli::collision::build_mask_for_asset;
use alveus_idle_cli::components::TilePosition;
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
