#![cfg(feature = "headless")]

use alveus_app::Screen;
use alveus_cleaning::{CleaningPlugin, PoopWheelbarrow};
use alveus_collision::CollisionMasks;
use alveus_headless::register_headless_types;
use alveus_stats::{
    EnclosureId, EnclosureStat, EnclosureStats, SavePath, StatTarget, StatsPlugin, WorsenStatEvent,
};
use alveus_types::Stat;
use bevy::prelude::*;
use bevy::remote::{BrpMessage, BrpSender};
use bevy::state::app::StatesPlugin;

fn headless_cleaning_brp_app(save_path: &str) -> App {
    let mut app = App::new();
    app.add_plugins(StatesPlugin);
    app.init_state::<Screen>();
    app.add_plugins(MinimalPlugins);
    app.init_resource::<ButtonInput<KeyCode>>();
    app.init_resource::<CollisionMasks>();
    app.insert_resource(SavePath(save_path.to_string()));
    app.add_plugins((StatsPlugin, CleaningPlugin, alveus_headless::CommandPlugin));
    app.add_plugins(bevy::remote::RemotePlugin::default());
    register_headless_types(&mut app);
    app.insert_resource(NextState::Pending(Screen::Gameplay));
    app
}

#[test]
fn brp_poop_wheelbarrow_is_queryable() {
    let save_path = "brp_clean_wheelbarrow.ron";
    let _ = std::fs::remove_file(save_path);

    let mut app = headless_cleaning_brp_app(save_path);
    app.update();

    assert_eq!(app.world().resource::<PoopWheelbarrow>().count(), 0);

    let sender = app.world().resource::<BrpSender>().clone();
    let (result_sender, result_receiver) = async_channel::bounded(1);
    sender
        .send_blocking(BrpMessage {
            method: "world.get_resources".to_string(),
            params: Some(serde_json::json!({
                "resource": "alveus_components::PoopWheelbarrow"
            })),
            sender: result_sender,
        })
        .expect("send brp message");

    app.update();

    let response = result_receiver.recv_blocking().expect("brp response");
    assert!(response.is_ok(), "expected ok response: {response:?}");

    let _ = std::fs::remove_file(save_path);
}

#[test]
fn brp_improve_stat_cleanliness_for_push_pop_enclosure() {
    let save_path = "brp_clean_improve.ron";
    let _ = std::fs::remove_file(save_path);

    let mut app = headless_cleaning_brp_app(save_path);
    app.update();

    let enc_entity = app
        .world_mut()
        .query_filtered::<Entity, With<EnclosureId>>()
        .iter(app.world())
        .find(|&e| *app.world().get::<EnclosureId>(e).unwrap() == EnclosureId::PushPopEnclosure)
        .expect("Push Pop enclosure");

    app.world_mut().trigger(WorsenStatEvent {
        target: StatTarget::Enclosure {
            id: EnclosureId::PushPopEnclosure,
            stat: EnclosureStat::Cleanliness,
        },
        amount: Stat(567),
    });
    assert_eq!(
        app.world()
            .get::<EnclosureStats>(enc_entity)
            .unwrap()
            .cleanliness,
        Stat(433)
    );

    let sender = app.world().resource::<BrpSender>().clone();
    let (result_sender, result_receiver) = async_channel::bounded(1);
    sender
        .send_blocking(BrpMessage {
            method: "world.trigger_event".to_string(),
            params: Some(serde_json::json!({
                "event": "alveus_headless::command::GameCommand",
                "value": {
                    "ImproveStat": {
                        "target": {
                            "Enclosure": {
                                "id": "PushPopEnclosure",
                                "stat": "Cleanliness"
                            }
                        },
                        "amount": 333
                    }
                }
            })),
            sender: result_sender,
        })
        .expect("send brp message");

    app.update();

    let response = result_receiver.recv_blocking().expect("brp response");
    assert!(response.is_ok(), "expected ok response: {response:?}");

    let stats = app.world().get::<EnclosureStats>(enc_entity).unwrap();
    assert_eq!(stats.cleanliness, Stat(766));

    let _ = std::fs::remove_file(save_path);
}
