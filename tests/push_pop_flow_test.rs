use alveus_app::Screen;
use alveus_cleaning::CleaningPlugin;
use alveus_components::TilePosition;
use alveus_configs::CARE_FEED_RESTORE;
use alveus_content::ItemId;
use alveus_interaction::{
    AnimalFedEvent, FeedAnimal, InteractionPlugin, PlayerSatchel, try_give_item,
};
use alveus_stats::{AnimalId, AnimalStats, SavePath, StatsPlugin};
use alveus_types::Stat;
use bevy::prelude::*;
use bevy::state::app::StatesPlugin;

#[test]
fn test_push_pop_feed_restores_hunger() {
    let save_path = "nonexistent_push_pop_feed.ron";
    let _ = std::fs::remove_file(save_path);

    let mut app = App::new();
    app.add_plugins(StatesPlugin);
    app.add_plugins(alveus_app::plugin);
    app.add_plugins(MinimalPlugins);
    app.init_resource::<ButtonInput<KeyCode>>();
    app.insert_resource(SavePath(save_path.to_string()));
    app.add_plugins((StatsPlugin, CleaningPlugin, InteractionPlugin));

    app.insert_resource(NextState::Pending(Screen::Gameplay));
    app.update();

    let push_pop_entity = app
        .world_mut()
        .query_filtered::<Entity, With<AnimalId>>()
        .iter(app.world())
        .find(|&e| *app.world().get::<AnimalId>(e).unwrap() == AnimalId::PushPop)
        .expect("Push Pop should exist");

    {
        let mut stats = app
            .world_mut()
            .get_mut::<AnimalStats>(push_pop_entity)
            .unwrap();
        stats.hunger = Stat(200);
    }

    let mut satchel = PlayerSatchel::default();
    try_give_item(&mut satchel, ItemId::TortoiseLeafyGreens).unwrap();
    app.insert_resource(satchel);

    app.world_mut().trigger(AnimalFedEvent {
        animal_id: AnimalId::PushPop,
        required_item: ItemId::TortoiseLeafyGreens,
        delta: CARE_FEED_RESTORE,
        dish_position: TilePosition { x: 8, y: 6 },
    });
    app.update();

    let stats = app.world().get::<AnimalStats>(push_pop_entity).unwrap();
    assert_eq!(stats.hunger, Stat(1000));
    assert!(app.world().resource::<PlayerSatchel>().is_empty());

    let _ = std::fs::remove_file(save_path);
}

#[test]
fn test_push_pop_feeding_dish_feed_animal_component() {
    let feed = FeedAnimal {
        animal_id: AnimalId::PushPop,
        required_item: ItemId::TortoiseLeafyGreens,
        delta: CARE_FEED_RESTORE,
        prompt: "Place leafy greens for Push Pop".to_string(),
    };

    assert_eq!(feed.animal_id, AnimalId::PushPop);
    assert_eq!(feed.required_item, ItemId::TortoiseLeafyGreens);
    assert_eq!(feed.delta, CARE_FEED_RESTORE);
}
