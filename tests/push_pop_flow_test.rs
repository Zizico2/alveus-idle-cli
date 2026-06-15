use bevy::prelude::*;
use bevy::state::app::StatesPlugin;
use alveus_idle_cli::content::{InteractionKind, ItemId, RoomObjectId};
use alveus_idle_cli::components::TilePosition;
use alveus_idle_cli::interaction::{
    try_feed_animal, try_give_item, AnimalFedEvent, PlayerSatchel, InteractionPlugin,
};
use alveus_idle_cli::screens::Screen;
use alveus_idle_cli::stats::{
    AnimalId, AnimalStat, AnimalStats, ImproveStatEvent, SavePath, StatTarget, StatsPlugin,
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
    try_feed_animal(&mut satchel, ItemId::TortoiseLeafyGreens).unwrap();

    app.world_mut().trigger(ImproveStatEvent {
        target: StatTarget::Animal {
            id: AnimalId::PushPop,
            stat: AnimalStat::Hunger,
        },
        amount: 1000,
    });

    app.world_mut().trigger(AnimalFedEvent {
        animal: AnimalId::PushPop,
        dish_position: TilePosition { x: 8, y: 6 },
    });

    let stats = app.world().get::<AnimalStats>(push_pop_entity).unwrap();
    assert_eq!(stats.hunger, 1000);
    assert!(satchel.item.is_none());

    let _ = std::fs::remove_file(save_path);
}

#[test]
fn test_feed_interaction_kind_matches_push_pop_dish() {
    use alveus_idle_cli::content::PUSH_POP_ENCLOSURE_OBJECTS;

    let dish = PUSH_POP_ENCLOSURE_OBJECTS
        .iter()
        .find(|o| o.object_id == RoomObjectId::PushPopFeedingDish)
        .expect("feeding dish should be defined");

    match dish.interaction.unwrap() {
        InteractionKind::FeedAnimal {
            animal_id,
            required_item,
            delta,
            ..
        } => {
            assert_eq!(animal_id, AnimalId::PushPop);
            assert_eq!(required_item, ItemId::TortoiseLeafyGreens);
            assert_eq!(delta, 1000);
        }
        _ => panic!("feeding dish should feed Push Pop"),
    }
}
