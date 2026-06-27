use bevy::prelude::*;
use bevy::state::app::StatesPlugin;
use alveus_idle_cli::components::TilePosition;
use alveus_idle_cli::content::ItemId;
use alveus_idle_cli::interaction::{
    try_give_item, AnimalFedEvent, FeedAnimal, InteractionPlugin, PlayerSatchel,
};
use alveus_idle_cli::screens::Screen;
use alveus_idle_cli::stats::{
    AnimalId, AnimalStat, AnimalStats, SavePath, StatsPlugin,
};

#[test]
fn test_push_pop_feed_restores_hunger() {
    let save_path = "nonexistent_push_pop_feed.ron";
    let _ = std::fs::remove_file(save_path);

    let mut app = App::new();
    app.add_plugins(StatesPlugin);
    app.init_state::<Screen>();
    app.add_plugins(MinimalPlugins);
    app.init_resource::<ButtonInput<KeyCode>>();
    app.insert_resource(SavePath(save_path.to_string()));
    app.add_plugins((StatsPlugin, InteractionPlugin));

    app.insert_resource(NextState::Pending(Screen::Gameplay));
    app.update();

    let push_pop_entity = app
        .world_mut()
        .query_filtered::<Entity, With<AnimalId>>()
        .iter(app.world())
        .find(|&e| {
            *app.world().get::<AnimalId>(e).unwrap() == AnimalId::PushPop
        })
        .expect("Push Pop should exist");

    {
        let mut stats = app
            .world_mut()
            .get_mut::<AnimalStats>(push_pop_entity)
            .unwrap();
        stats.hunger = 200;
    }

    let mut satchel = PlayerSatchel::default();
    try_give_item(&mut satchel, ItemId::TortoiseLeafyGreens).unwrap();
    app.insert_resource(satchel);

    app.world_mut().trigger(AnimalFedEvent {
        animal_id: AnimalId::PushPop,
        required_item: ItemId::TortoiseLeafyGreens,
        stat: AnimalStat::Hunger,
        delta: 1000,
        dish_position: TilePosition { x: 8, y: 6 },
    });
    app.update();

    let stats = app.world().get::<AnimalStats>(push_pop_entity).unwrap();
    assert_eq!(stats.hunger, 1000);
    assert!(app.world().resource::<PlayerSatchel>().item.is_none());

    let _ = std::fs::remove_file(save_path);
}

#[test]
fn test_push_pop_feeding_dish_feed_animal_component() {
    let feed = FeedAnimal {
        animal_id: AnimalId::PushPop,
        required_item: ItemId::TortoiseLeafyGreens,
        stat: AnimalStat::Hunger,
        delta: 1000,
        prompt: "Place leafy greens for Push Pop".to_string(),
    };

    assert_eq!(feed.animal_id, AnimalId::PushPop);
    assert_eq!(feed.required_item, ItemId::TortoiseLeafyGreens);
    assert_eq!(feed.stat, AnimalStat::Hunger);
    assert_eq!(feed.delta, 1000);
}
