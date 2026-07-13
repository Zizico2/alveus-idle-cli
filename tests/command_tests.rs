mod common;

use alveus_app::Screen;
use alveus_command::GameCommand;
use alveus_components::{
    LastPickupMessage, MovementController, MovementDuration, MovementIntent, Player,
};
use alveus_interaction::{InteractionPlugin, PlayerSatchel};
use alveus_stats::{AnimalId, AnimalStat, AnimalStats, StatTarget};
use alveus_types::Stat;
use bevy::prelude::*;

#[test]
fn move_command_sets_player_intent() {
    let save_path = "command_test_move.ron";
    let mut app = common::minimal_stats_app(save_path);

    app.world_mut().spawn((
        Player,
        MovementController::default(),
        alveus_components::CurrentTilePosition::default(),
        alveus_components::DesiredTilePosition::default(),
        MovementDuration(Timer::from_seconds(
            alveus_configs::PLAYER_MOVE_DURATION_SECS,
            TimerMode::Once,
        )),
    ));

    app.world_mut()
        .trigger(GameCommand::Move(MovementIntent::Up));
    app.update();

    let intent = app
        .world_mut()
        .query_filtered::<&MovementController, With<Player>>()
        .single(app.world())
        .expect("player movement controller")
        .intent;
    assert_eq!(intent, Some(MovementIntent::Up));

    common::cleanup_save(save_path);
}

#[test]
fn skip_splash_command_changes_screen() {
    let save_path = "command_test_skip.ron";
    let mut app = common::minimal_stats_app(save_path);

    app.insert_resource(State::new(Screen::Splash));
    app.world_mut().trigger(GameCommand::SkipSplash);
    app.update();

    assert_eq!(
        *app.world().resource::<State<Screen>>().get(),
        Screen::Title
    );

    common::cleanup_save(save_path);
}

#[test]
fn worsen_stat_command_updates_animal() {
    let save_path = "command_test_improve.ron";
    let mut app = common::minimal_stats_app(save_path);

    let polly = app
        .world_mut()
        .query_filtered::<Entity, With<AnimalId>>()
        .iter(app.world())
        .find(|entity| {
            app.world()
                .get::<AnimalId>(*entity)
                .is_some_and(|id| *id == AnimalId::Polly)
        })
        .expect("polly entity");

    app.world_mut().trigger(GameCommand::WorsenStat {
        target: StatTarget::Animal {
            id: AnimalId::Polly,
            stat: AnimalStat::Hunger,
        },
        amount: Stat(400),
    });
    app.update();

    let stats = app
        .world()
        .get::<alveus_stats::AnimalStats>(polly)
        .expect("polly stats");
    assert_eq!(stats.hunger, Stat(600));

    common::cleanup_save(save_path);
}

#[test]
fn improve_stat_command_updates_animal() {
    let save_path = "command_test_improve_stat.ron";
    let mut app = common::minimal_stats_app(save_path);

    let polly = app
        .world_mut()
        .query_filtered::<Entity, With<AnimalId>>()
        .iter(app.world())
        .find(|entity| {
            app.world()
                .get::<AnimalId>(*entity)
                .is_some_and(|id| *id == AnimalId::Polly)
        })
        .expect("polly entity");

    app.world_mut().trigger(GameCommand::WorsenStat {
        target: StatTarget::Animal {
            id: AnimalId::Polly,
            stat: AnimalStat::Hunger,
        },
        amount: Stat(400),
    });
    app.update();

    app.world_mut().trigger(GameCommand::ImproveStat {
        target: StatTarget::Animal {
            id: AnimalId::Polly,
            stat: AnimalStat::Hunger,
        },
        amount: Stat(250),
    });
    app.update();

    let stats = app.world().get::<AnimalStats>(polly).expect("polly stats");
    assert_eq!(stats.hunger, Stat(850));

    common::cleanup_save(save_path);
}

#[test]
fn drop_item_empty_satchel_is_noop() {
    let save_path = "command_test_drop_empty.ron";
    let mut app = common::minimal_stats_app(save_path);
    app.add_plugins(InteractionPlugin);

    app.insert_resource(LastPickupMessage {
        text: Some("sentinel".to_string()),
        timer: Timer::from_seconds(2.5, TimerMode::Once),
    });
    assert!(app.world().resource::<PlayerSatchel>().is_empty());

    app.world_mut().trigger(GameCommand::DropItem);
    app.update();

    assert_eq!(
        app.world().resource::<LastPickupMessage>().text.as_deref(),
        Some("sentinel")
    );

    common::cleanup_save(save_path);
}
