#![cfg(feature = "headless")]

use alveus_idle_cli::headless::{register_headless_types, GameCommand};
use alveus_idle_cli::screens::Screen;
use alveus_idle_cli::stats::{SavePath, StatsPlugin};
use bevy::prelude::*;
use bevy::state::app::StatesPlugin;
use bevy::remote::{BrpMessage, BrpSender};

#[test]
fn registered_types_include_game_command_and_screen() {
    let mut app = App::new();
    register_headless_types(&mut app);

    let registry = app.world().resource::<AppTypeRegistry>();
    let registry = registry.read();

    assert!(registry.get(std::any::TypeId::of::<GameCommand>()).is_some());
    assert!(registry.get(std::any::TypeId::of::<Screen>()).is_some());
}

#[test]
fn brp_skip_splash_command_changes_screen() {
    let save_path = "brp_test_skip.ron";
    let mut app = App::new();
    app.add_plugins(StatesPlugin);
    app.init_state::<Screen>();
    app.add_plugins(MinimalPlugins);
    app.insert_resource(SavePath(save_path.to_string()));
    app.add_plugins((StatsPlugin, alveus_idle_cli::headless::CommandPlugin));
    app.add_plugins(bevy::remote::RemotePlugin::default());
    register_headless_types(&mut app);
    app.insert_resource(State::new(Screen::Splash));
    app.update();

    let sender = app.world().resource::<BrpSender>().clone();
    let (result_sender, result_receiver) = async_channel::bounded(1);
    sender
        .send_blocking(BrpMessage {
            method: "world.trigger_event".to_string(),
            params: Some(serde_json::json!({
                "event": "alveus_idle_cli::headless::command::GameCommand",
                "value": "SkipSplash"
            })),
            sender: result_sender,
        })
        .expect("send brp message");

    app.update();

    let response = result_receiver.recv_blocking().expect("brp response");
    assert!(response.is_ok(), "expected ok response: {response:?}");
    assert_eq!(*app.world().resource::<State<Screen>>().get(), Screen::Title);

    let _ = std::fs::remove_file(save_path);
}
