mod common;

use alveus_app::{Menu, Screen};
use alveus_command::{GameCommand, StepRequest};
use alveus_components::{
    LastPickupMessage, MovementController, MovementDuration, MovementIntent, Player,
};
use alveus_interaction::{InteractionPlugin, PlayerSatchel};
use alveus_stats::{AnimalId, AnimalStat, AnimalStats, StatTarget};
use alveus_types::Stat;
use bevy::audio::Volume;
use bevy::prelude::*;

fn spawn_test_player(app: &mut App) {
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
}

fn player_intent(app: &mut App) -> Option<MovementIntent> {
    app.world_mut()
        .query_filtered::<&MovementController, With<Player>>()
        .single(app.world())
        .expect("player movement controller")
        .intent
}

#[test]
fn move_command_sets_player_intent() {
    let save_path = "command_test_move.ron";
    let mut app = common::minimal_stats_app(save_path);
    spawn_test_player(&mut app);

    app.world_mut()
        .trigger(GameCommand::Move(MovementIntent::Up));
    app.update();

    assert_eq!(player_intent(&mut app), Some(MovementIntent::Up));

    common::cleanup_save(save_path);
}

#[test]
fn move_then_move_stop_before_one_update_clears_intent() {
    let save_path = "command_test_move_stop_batch.ron";
    let mut app = common::minimal_stats_app(save_path);
    spawn_test_player(&mut app);

    app.world_mut()
        .trigger(GameCommand::Move(MovementIntent::Right));
    app.world_mut().trigger(GameCommand::MoveStop);
    app.update();

    assert_eq!(player_intent(&mut app), None);

    common::cleanup_save(save_path);
}

#[derive(Resource, Default)]
struct OpenSettingsViaCommands(bool);

fn queue_open_settings_via_commands(
    mut commands: Commands,
    mut queued: ResMut<OpenSettingsViaCommands>,
) {
    if queued.0 {
        return;
    }
    commands.trigger(GameCommand::OpenSettings);
    queued.0 = true;
}

#[test]
fn nested_game_command_via_commands_opens_settings_in_one_update() {
    let save_path = "command_test_nested_open_settings.ron";
    let mut app = common::minimal_stats_app(save_path);
    app.init_resource::<OpenSettingsViaCommands>();
    app.add_systems(Update, queue_open_settings_via_commands);

    app.update();

    assert_eq!(*app.world().resource::<State<Menu>>().get(), Menu::Settings);

    common::cleanup_save(save_path);
}

#[test]
fn fatal_error_rejects_gameplay_commands_but_accepts_advance_frames() {
    let save_path = "command_test_fatal_error_allowlist.ron";
    let mut app = common::minimal_stats_app(save_path);
    spawn_test_player(&mut app);

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

    let hunger_before = app.world().get::<AnimalStats>(polly).unwrap().hunger;
    let intent_before = player_intent(&mut app);

    app.insert_resource(State::new(Screen::FatalError));
    app.world_mut()
        .trigger(GameCommand::Move(MovementIntent::Up));
    app.world_mut().trigger(GameCommand::WorsenStat {
        target: StatTarget::Animal {
            id: AnimalId::Polly,
            stat: AnimalStat::Hunger,
        },
        amount: Stat(400),
    });
    app.world_mut().trigger(GameCommand::AdvanceFrames(3));
    app.update();

    assert_eq!(player_intent(&mut app), intent_before);
    assert_eq!(
        app.world().get::<AnimalStats>(polly).unwrap().hunger,
        hunger_before
    );
    assert_eq!(app.world().resource::<StepRequest>().pending, 3);

    common::cleanup_save(save_path);
}

#[test]
fn move_stop_clears_existing_intent() {
    let save_path = "command_test_move_stop_clear.ron";
    let mut app = common::minimal_stats_app(save_path);
    spawn_test_player(&mut app);

    app.world_mut()
        .trigger(GameCommand::Move(MovementIntent::Down));
    app.update();
    assert_eq!(player_intent(&mut app), Some(MovementIntent::Down));

    app.world_mut().trigger(GameCommand::MoveStop);
    app.update();
    assert_eq!(player_intent(&mut app), None);

    common::cleanup_save(save_path);
}

#[test]
fn pause_toggle_opens_and_closes_pause_menu() {
    let save_path = "command_test_pause_toggle.ron";
    let mut app = common::minimal_stats_app(save_path);

    app.world_mut().trigger(GameCommand::PauseToggle);
    app.update();
    assert_eq!(*app.world().resource::<State<Menu>>().get(), Menu::Pause);

    app.world_mut().trigger(GameCommand::PauseToggle);
    app.update();
    assert_eq!(*app.world().resource::<State<Menu>>().get(), Menu::None);

    common::cleanup_save(save_path);
}

#[test]
fn menu_flow_commands_open_settings_credits_and_back() {
    let save_path = "command_test_menu_flow.ron";
    let mut app = common::minimal_stats_app(save_path);

    app.world_mut().trigger(GameCommand::OpenSettings);
    app.update();
    assert_eq!(*app.world().resource::<State<Menu>>().get(), Menu::Settings);

    app.world_mut().trigger(GameCommand::Back);
    app.update();
    assert_eq!(*app.world().resource::<State<Menu>>().get(), Menu::Pause);

    app.world_mut().trigger(GameCommand::OpenCredits);
    app.update();
    assert_eq!(*app.world().resource::<State<Menu>>().get(), Menu::Credits);

    app.world_mut().trigger(GameCommand::Back);
    app.update();
    assert_eq!(*app.world().resource::<State<Menu>>().get(), Menu::Main);

    common::cleanup_save(save_path);
}

#[test]
fn adjust_volume_clamps_between_zero_and_three() {
    let save_path = "command_test_adjust_volume.ron";
    let mut app = common::minimal_stats_app(save_path);
    app.insert_resource(GlobalVolume {
        volume: Volume::Linear(1.5),
    });

    app.world_mut()
        .trigger(GameCommand::AdjustVolume { delta: 2.0 });
    app.update();
    assert!((app.world().resource::<GlobalVolume>().volume.to_linear() - 3.0).abs() < f32::EPSILON);

    app.world_mut()
        .trigger(GameCommand::AdjustVolume { delta: -10.0 });
    app.update();
    assert!((app.world().resource::<GlobalVolume>().volume.to_linear()).abs() < f32::EPSILON);

    common::cleanup_save(save_path);
}

#[test]
fn advance_frames_saturating_adds_to_step_request() {
    let save_path = "command_test_advance_frames.ron";
    let mut app = common::minimal_stats_app(save_path);

    app.world_mut().trigger(GameCommand::AdvanceFrames(2));
    app.world_mut()
        .trigger(GameCommand::AdvanceFrames(u32::MAX));
    app.update();

    assert_eq!(
        app.world().resource::<StepRequest>().pending,
        2_u32.saturating_add(u32::MAX)
    );

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
