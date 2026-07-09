#![cfg(feature = "headless")]

use alveus_headless::register_headless_types;
use alveus_app::Screen;
use alveus_stats::{SavePath, StatsPlugin};
use bevy::prelude::*;
use bevy::remote::{BrpMessage, BrpSender};
use bevy::state::app::StatesPlugin;
use serde_json::json;

#[test]
fn stdio_json_line_triggers_game_command() {
    let save_path = "stdio_test_skip.ron";
    let mut app = App::new();
    app.add_plugins(StatesPlugin);
    app.init_state::<Screen>();
    app.add_plugins(MinimalPlugins);
    app.insert_resource(SavePath(save_path.to_string()));
    app.add_plugins((StatsPlugin, alveus_headless::CommandPlugin));
    app.add_plugins(bevy::remote::RemotePlugin::default());
    register_headless_types(&mut app);
    app.insert_resource(State::new(Screen::Splash));
    app.update();

    let sender = app.world().resource::<BrpSender>().clone();
    let request = json!({
        "jsonrpc": "2.0",
        "method": "world.trigger_event",
        "id": 1,
        "params": {
            "event": "alveus_headless::command::GameCommand",
            "value": "SkipSplash"
        }
    });

    let (result_sender, result_receiver) = async_channel::bounded(1);
    sender
        .send_blocking(BrpMessage {
            method: request["method"].as_str().unwrap().to_string(),
            params: request.get("params").cloned(),
            sender: result_sender,
        })
        .expect("send brp request");

    app.update();

    let response = result_receiver.recv_blocking().expect("response");
    assert!(
        response.is_ok(),
        "stdio-equivalent request failed: {response:?}"
    );
    assert_eq!(
        *app.world().resource::<State<Screen>>().get(),
        Screen::Title
    );

    let _ = std::fs::remove_file(save_path);
}
