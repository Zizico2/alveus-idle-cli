mod common;

use alveus_components::{MovementController, MovementDuration, MovementIntent, Player};
use alveus_headless::GameCommand;
use alveus_app::Screen;
use alveus_stats::{AnimalId, AnimalStat, StatTarget};
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
        amount: 400,
    });
    app.update();

    let stats = app
        .world()
        .get::<alveus_stats::AnimalStats>(polly)
        .expect("polly stats");
    assert_eq!(stats.hunger, 600);

    common::cleanup_save(save_path);
}
