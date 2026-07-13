#![cfg(feature = "headless")]

use alveus_app::{Menu, Screen};
use alveus_command::GameCommand;
use alveus_components::{
    CareFeedbackEvent, CurrentTilePosition, Interactable, LastPickupMessage, Player, TilePosition,
};
use alveus_configs::{CARE_CLEAN_RESTORE, CARE_ENRICH_RESTORE, CARE_FEED_RESTORE};
use alveus_content::ItemId;
use alveus_interaction::{
    ActiveInteractionTarget, CareMenuState, CleanAnimal, EnrichAnimal, FeedAnimal, GiveItem,
    InteractionPlugin, MiniChore, OpenMenu, PlayerSatchel, care_outcome_message, satchel_contains,
    try_give_item,
};
use alveus_reflect::register_agent_types;
use alveus_stats::{
    AnimalId, AnimalStat, AnimalStats, EnclosureId, EnclosureStats, SavePath, StatsPlugin,
};
use alveus_types::{CareMenuId, ChoreId, Stat};
use bevy::prelude::*;
use bevy::remote::{BrpMessage, BrpResult, BrpSender};
use bevy::state::app::StatesPlugin;

#[derive(Resource, Default)]
struct CapturedCareFeedback(Option<String>);

fn capture_care_feedback(
    trigger: On<CareFeedbackEvent>,
    mut captured: ResMut<CapturedCareFeedback>,
) {
    captured.0 = Some(trigger.event().message.clone());
}

fn brp_request(app: &mut App, method: &str, params: Option<serde_json::Value>) -> BrpResult {
    let (result_sender, result_receiver) = async_channel::bounded(1);
    {
        let sender = app.world().resource::<BrpSender>();
        sender
            .send_blocking(BrpMessage {
                method: method.to_string(),
                params,
                sender: result_sender,
            })
            .expect("send brp message");
    }

    app.update();

    result_receiver.recv_blocking().expect("brp response")
}

fn try_trigger_game_command(app: &mut App, value: serde_json::Value) -> BrpResult {
    brp_request(
        app,
        "world.trigger_event",
        Some(serde_json::json!({
            "event": "alveus_headless::command::GameCommand",
            "value": value
        })),
    )
}

fn trigger_game_command(app: &mut App, value: serde_json::Value) {
    let response = try_trigger_game_command(app, value);
    assert!(response.is_ok(), "expected ok response: {response:?}");
}

fn care_brp_app(save_path: &str) -> App {
    let _ = std::fs::remove_file(save_path);
    let mut app = App::new();
    app.add_plugins(StatesPlugin);
    app.add_plugins(MinimalPlugins);
    app.add_plugins(alveus_app::plugin);
    app.init_resource::<ButtonInput<KeyCode>>();
    app.insert_resource(SavePath(save_path.to_string()));
    app.init_resource::<CapturedCareFeedback>();
    app.add_plugins((
        StatsPlugin,
        alveus_cleaning::CleaningPlugin,
        InteractionPlugin,
        alveus_command::CommandPlugin,
    ));
    app.add_plugins(bevy::remote::RemotePlugin::default());
    register_agent_types(&mut app);
    app.add_observer(capture_care_feedback);
    app.world_mut()
        .spawn((Player, CurrentTilePosition(TilePosition { x: 0, y: 0 })));
    app.world_mut()
        .resource_mut::<NextState<Screen>>()
        .set(Screen::Gameplay);
    app.update();
    app
}

#[test]
fn registered_types_include_game_command_and_screen() {
    let mut app = App::new();
    register_agent_types(&mut app);

    let registry = app.world().resource::<AppTypeRegistry>();
    let registry = registry.read();

    assert!(
        registry
            .get(std::any::TypeId::of::<GameCommand>())
            .is_some()
    );
    assert!(registry.get(std::any::TypeId::of::<Screen>()).is_some());
}

#[test]
fn care_menu_state_keeps_its_existing_brp_resource_path() {
    use bevy::reflect::TypePath;

    assert_eq!(
        CareMenuState::type_path(),
        "alveus_interaction::CareMenuState"
    );

    let save_path = "brp_test_care_menu_resource_path.ron";
    let mut app = care_brp_app(save_path);
    let response = brp_request(
        &mut app,
        "world.get_resources",
        Some(serde_json::json!({
            "resource": "alveus_interaction::CareMenuState"
        })),
    )
    .expect("existing CareMenuState BRP resource path remains queryable");
    let value = response.get("value").unwrap_or(&response);
    assert_eq!(value["cursor"], serde_json::json!(0));
    assert_eq!(value["menu_id"], serde_json::Value::Null);

    let _ = std::fs::remove_file(save_path);
}

#[test]
fn brp_skip_splash_command_changes_screen() {
    let save_path = "brp_test_skip.ron";
    let mut app = App::new();
    app.add_plugins(StatesPlugin);
    app.add_plugins(alveus_app::plugin);
    app.add_plugins(MinimalPlugins);
    app.insert_resource(SavePath(save_path.to_string()));
    app.add_plugins((StatsPlugin, alveus_command::CommandPlugin));
    app.add_plugins(bevy::remote::RemotePlugin::default());
    register_agent_types(&mut app);
    app.insert_resource(State::new(Screen::Splash));
    app.update();

    trigger_game_command(&mut app, serde_json::json!("SkipSplash"));
    assert_eq!(
        *app.world().resource::<State<Screen>>().get(),
        Screen::Title
    );

    let _ = std::fs::remove_file(save_path);
}

#[test]
fn brp_care_menu_moves_cursor_and_confirms_selection() {
    let save_path = "brp_test_care_menu.ron";
    let mut app = care_brp_app(save_path);

    app.world_mut().spawn((
        OpenMenu {
            menu_id: CareMenuId::Fridge,
            prompt: "Open fridge".to_string(),
        },
        TilePosition { x: 0, y: 0 },
        Interactable,
    ));
    app.update();

    trigger_game_command(&mut app, serde_json::json!("Interact"));
    assert_eq!(
        *app.world().resource::<State<Menu>>().get(),
        Menu::CareItemPicker
    );

    trigger_game_command(&mut app, serde_json::json!({ "Move": "Down" }));
    assert_eq!(app.world().resource::<CareMenuState>().cursor, 1);

    trigger_game_command(&mut app, serde_json::json!("Continue"));
    let satchel = app.world().resource::<PlayerSatchel>();
    assert!(satchel_contains(satchel, ItemId::TortoiseLeafyGreens));

    let _ = std::fs::remove_file(save_path);
}

#[test]
fn brp_world_interaction_commands_are_blocked_by_overlay_menu() {
    let save_path = "brp_test_overlay_blocks_world_interaction.ron";
    let mut app = care_brp_app(save_path);

    let pickup = app
        .world_mut()
        .spawn((
            GiveItem {
                item_id: ItemId::MiniMirror,
                prompt: "Pick up mirror".to_string(),
            },
            TilePosition { x: 0, y: 0 },
            Interactable,
        ))
        .id();
    try_give_item(
        &mut app.world_mut().resource_mut::<PlayerSatchel>(),
        ItemId::RawVeggieTub,
    )
    .unwrap();
    app.world_mut()
        .resource_mut::<NextState<Menu>>()
        .set(Menu::Pause);
    app.update();
    app.world_mut()
        .resource_mut::<ActiveInteractionTarget>()
        .interactable = Some(pickup);

    trigger_game_command(&mut app, serde_json::json!("Interact"));
    trigger_game_command(&mut app, serde_json::json!("DropItem"));

    let satchel = app.world().resource::<PlayerSatchel>();
    assert_eq!(satchel.slots[0], Some(ItemId::RawVeggieTub));
    assert!(!satchel_contains(satchel, ItemId::MiniMirror));

    let _ = std::fs::remove_file(save_path);
}

#[test]
fn brp_mini_chore_transforms_required_item() {
    let save_path = "brp_test_mini_chore.ron";
    let mut app = care_brp_app(save_path);

    try_give_item(
        &mut app.world_mut().resource_mut::<PlayerSatchel>(),
        ItemId::RawVeggieTub,
    )
    .unwrap();
    app.world_mut().spawn((
        MiniChore {
            chore_id: ChoreId::ChopVeggies,
            required_item: Some(ItemId::RawVeggieTub),
            output_item: Some(ItemId::PreparedVeggieDiet),
            prompt: "Chop veggies".to_string(),
        },
        TilePosition { x: 0, y: 0 },
        Interactable,
    ));
    app.update();

    trigger_game_command(&mut app, serde_json::json!("Interact"));

    let satchel = app.world().resource::<PlayerSatchel>();
    assert!(satchel_contains(satchel, ItemId::PreparedVeggieDiet));
    assert!(!satchel_contains(satchel, ItemId::RawVeggieTub));

    let _ = std::fs::remove_file(save_path);
}

#[test]
fn brp_enrich_interaction_restores_happiness() {
    let save_path = "brp_test_enrich.ron";
    let mut app = care_brp_app(save_path);

    let push_pop = app
        .world_mut()
        .query_filtered::<Entity, With<AnimalId>>()
        .iter(app.world())
        .find(|&entity| app.world().get::<AnimalId>(entity) == Some(&AnimalId::PushPop))
        .expect("Push Pop stats entity");
    app.world_mut()
        .get_mut::<AnimalStats>(push_pop)
        .unwrap()
        .happiness = Stat(0);

    app.world_mut().spawn((
        EnrichAnimal {
            animal_id: AnimalId::PushPop,
            required_item: None,
            delta: CARE_ENRICH_RESTORE,
            prompt: "Scatter hay".to_string(),
        },
        TilePosition { x: 0, y: 0 },
        Interactable,
    ));
    app.update();

    trigger_game_command(&mut app, serde_json::json!("Interact"));

    assert_eq!(
        app.world().get::<AnimalStats>(push_pop).unwrap().happiness,
        CARE_ENRICH_RESTORE.0
    );

    let text = app
        .world()
        .resource::<CapturedCareFeedback>()
        .0
        .as_deref()
        .expect("care feedback");
    assert_eq!(
        text,
        care_outcome_message(AnimalId::PushPop, AnimalStat::Happiness)
    );
    assert!(text.contains("Enriched"), "{text}");
    assert!(!text.contains("Cleaned"), "{text}");
    assert!(app.world().resource::<LastPickupMessage>().text.is_none());

    let _ = std::fs::remove_file(save_path);
}

#[test]
fn brp_clean_interaction_says_cleaned_not_enriched() {
    let save_path = "brp_test_clean_feedback.ron";
    let mut app = care_brp_app(save_path);

    let enc_entity = app
        .world_mut()
        .query_filtered::<Entity, With<EnclosureId>>()
        .iter(app.world())
        .find(|&entity| {
            app.world().get::<EnclosureId>(entity) == Some(&EnclosureId::PushPopEnclosure)
        })
        .expect("Push Pop enclosure");
    app.world_mut()
        .get_mut::<EnclosureStats>(enc_entity)
        .unwrap()
        .cleanliness = Stat(50);

    app.world_mut().spawn((
        CleanAnimal {
            animal_id: AnimalId::PushPop,
            required_item: None,
            delta: CARE_CLEAN_RESTORE,
            prompt: "Sweep nesting".to_string(),
        },
        TilePosition { x: 0, y: 0 },
        Interactable,
    ));
    app.update();

    trigger_game_command(&mut app, serde_json::json!("Interact"));

    assert_eq!(
        app.world()
            .get::<EnclosureStats>(enc_entity)
            .unwrap()
            .cleanliness,
        CARE_CLEAN_RESTORE.0
    );

    let text = app
        .world()
        .resource::<CapturedCareFeedback>()
        .0
        .as_deref()
        .expect("care feedback");
    assert_eq!(
        text,
        care_outcome_message(AnimalId::PushPop, AnimalStat::Cleanliness)
    );
    assert!(text.contains("Cleaned"), "{text}");
    assert!(!text.contains("Enriched"), "{text}");
    assert!(app.world().resource::<LastPickupMessage>().text.is_none());

    let _ = std::fs::remove_file(save_path);
}

#[test]
fn brp_feed_interaction_says_fed_and_consumes_item() {
    let save_path = "brp_test_feed_feedback.ron";
    let mut app = care_brp_app(save_path);

    try_give_item(
        &mut app.world_mut().resource_mut::<PlayerSatchel>(),
        ItemId::TortoiseLeafyGreens,
    )
    .unwrap();

    let push_pop = app
        .world_mut()
        .query_filtered::<Entity, With<AnimalId>>()
        .iter(app.world())
        .find(|&entity| app.world().get::<AnimalId>(entity) == Some(&AnimalId::PushPop))
        .expect("Push Pop stats entity");
    app.world_mut()
        .get_mut::<AnimalStats>(push_pop)
        .unwrap()
        .hunger = Stat(100);

    app.world_mut().spawn((
        FeedAnimal {
            animal_id: AnimalId::PushPop,
            required_item: ItemId::TortoiseLeafyGreens,
            delta: CARE_FEED_RESTORE,
            prompt: "Fill dish".to_string(),
        },
        TilePosition { x: 0, y: 0 },
        Interactable,
    ));
    app.update();

    trigger_game_command(&mut app, serde_json::json!("Interact"));

    assert_eq!(
        app.world().get::<AnimalStats>(push_pop).unwrap().hunger,
        CARE_FEED_RESTORE.0
    );
    assert!(!satchel_contains(
        app.world().resource::<PlayerSatchel>(),
        ItemId::TortoiseLeafyGreens
    ));

    let text = app
        .world()
        .resource::<CapturedCareFeedback>()
        .0
        .as_deref()
        .expect("care feedback");
    assert_eq!(
        text,
        care_outcome_message(AnimalId::PushPop, AnimalStat::Hunger)
    );
    assert!(text.contains("Fed"), "{text}");
    assert!(!text.contains("Cleaned"), "{text}");
    assert!(!text.contains("Enriched"), "{text}");
    assert!(app.world().resource::<LastPickupMessage>().text.is_none());

    let _ = std::fs::remove_file(save_path);
}

#[test]
fn brp_improve_stat_accepts_stat_amount() {
    let save_path = "brp_test_improve_stat.ron";
    let mut app = care_brp_app(save_path);

    let push_pop = app
        .world_mut()
        .query_filtered::<Entity, With<AnimalId>>()
        .iter(app.world())
        .find(|&entity| app.world().get::<AnimalId>(entity) == Some(&AnimalId::PushPop))
        .expect("Push Pop stats entity");

    app.world_mut()
        .get_mut::<AnimalStats>(push_pop)
        .unwrap()
        .hunger = Stat(500);

    // Reflect serializes the Stat newtype as a bare u32 on the BRP wire.
    trigger_game_command(
        &mut app,
        serde_json::json!({
            "ImproveStat": {
                "target": {
                    "Animal": {
                        "id": "PushPop",
                        "stat": "Hunger"
                    }
                },
                "amount": 250
            }
        }),
    );

    assert_eq!(
        app.world().get::<AnimalStats>(push_pop).unwrap().hunger,
        Stat(750)
    );

    let _ = std::fs::remove_file(save_path);
}

#[test]
fn brp_improve_stat_rejects_tuple_map_amount() {
    let save_path = "brp_test_improve_map_reject.ron";
    let mut app = care_brp_app(save_path);

    let (result_sender, result_receiver) = async_channel::bounded(1);
    {
        let sender = app.world().resource::<BrpSender>();
        sender
            .send_blocking(BrpMessage {
                method: "world.trigger_event".to_string(),
                params: Some(serde_json::json!({
                    "event": "alveus_headless::command::GameCommand",
                    "value": {
                        "ImproveStat": {
                            "target": {
                                "Animal": {
                                    "id": "PushPop",
                                    "stat": "Hunger"
                                }
                            },
                            "amount": { "0": 250 }
                        }
                    }
                })),
                sender: result_sender,
            })
            .expect("send brp message");
    }

    app.update();

    let response = result_receiver.recv_blocking().expect("brp response");
    assert!(
        response.is_err(),
        "tuple-map amount must be rejected for Stat newtype wire: {response:?}"
    );

    let _ = std::fs::remove_file(save_path);
}

#[test]
fn brp_worsen_stat_accepts_stat_amount() {
    let save_path = "brp_test_worsen_stat.ron";
    let mut app = care_brp_app(save_path);

    let push_pop = app
        .world_mut()
        .query_filtered::<Entity, With<AnimalId>>()
        .iter(app.world())
        .find(|&entity| app.world().get::<AnimalId>(entity) == Some(&AnimalId::PushPop))
        .expect("Push Pop stats entity");

    app.world_mut()
        .get_mut::<AnimalStats>(push_pop)
        .unwrap()
        .hunger = Stat(500);

    trigger_game_command(
        &mut app,
        serde_json::json!({
            "WorsenStat": {
                "target": {
                    "Animal": {
                        "id": "PushPop",
                        "stat": "Hunger"
                    }
                },
                "amount": 250
            }
        }),
    );

    assert_eq!(
        app.world().get::<AnimalStats>(push_pop).unwrap().hunger,
        Stat(250)
    );

    let _ = std::fs::remove_file(save_path);
}

#[test]
fn brp_worsen_stat_rejects_tuple_map_amount() {
    let save_path = "brp_test_worsen_map_reject.ron";
    let mut app = care_brp_app(save_path);

    let response = try_trigger_game_command(
        &mut app,
        serde_json::json!({
            "WorsenStat": {
                "target": {
                    "Animal": {
                        "id": "PushPop",
                        "stat": "Hunger"
                    }
                },
                "amount": { "0": 250 }
            }
        }),
    );

    assert!(
        response.is_err(),
        "tuple-map amount must be rejected for Stat newtype wire: {response:?}"
    );

    let _ = std::fs::remove_file(save_path);
}

#[test]
fn brp_world_query_serializes_stat_fields_as_bare_numbers() {
    let save_path = "brp_test_stat_query_shape.ron";
    let mut app = care_brp_app(save_path);

    let push_pop = app
        .world_mut()
        .query_filtered::<Entity, With<AnimalId>>()
        .iter(app.world())
        .find(|&entity| app.world().get::<AnimalId>(entity) == Some(&AnimalId::PushPop))
        .expect("Push Pop stats entity");
    app.world_mut()
        .get_mut::<AnimalStats>(push_pop)
        .unwrap()
        .hunger = Stat(123);
    app.world_mut()
        .get_mut::<AnimalStats>(push_pop)
        .unwrap()
        .happiness = Stat(456);

    let push_pop_enclosure = app
        .world_mut()
        .query_filtered::<Entity, With<EnclosureId>>()
        .iter(app.world())
        .find(|&entity| {
            app.world().get::<EnclosureId>(entity) == Some(&EnclosureId::PushPopEnclosure)
        })
        .expect("Push Pop enclosure stats entity");
    app.world_mut()
        .get_mut::<EnclosureStats>(push_pop_enclosure)
        .unwrap()
        .cleanliness = Stat(321);

    let animal_rows = brp_request(
        &mut app,
        "world.query",
        Some(serde_json::json!({
            "data": {
                "components": ["alveus_stats::AnimalStats"],
                "has": []
            },
            "filter": {
                "with": ["alveus_types::AnimalId"]
            }
        })),
    )
    .expect("animal stats query succeeds");
    let animal_components = animal_rows
        .as_array()
        .expect("animal query rows")
        .iter()
        .map(|row| &row["components"]["alveus_stats::AnimalStats"])
        .find(|component| **component == serde_json::json!({"hunger": 123, "happiness": 456}))
        .expect("Push Pop AnimalStats payload");
    assert_eq!(
        animal_components,
        &serde_json::json!({"hunger": 123, "happiness": 456})
    );

    let enclosure_rows = brp_request(
        &mut app,
        "world.query",
        Some(serde_json::json!({
            "data": {
                "components": ["alveus_stats::EnclosureStats"],
                "has": []
            },
            "filter": {
                "with": ["alveus_types::EnclosureId"]
            }
        })),
    )
    .expect("enclosure stats query succeeds");
    let enclosure_components = enclosure_rows
        .as_array()
        .expect("enclosure query rows")
        .iter()
        .map(|row| &row["components"]["alveus_stats::EnclosureStats"])
        .find(|component| **component == serde_json::json!({"cleanliness": 321}))
        .expect("Push Pop EnclosureStats payload");
    assert_eq!(
        enclosure_components,
        &serde_json::json!({"cleanliness": 321})
    );

    let _ = std::fs::remove_file(save_path);
}

#[test]
fn brp_registry_schema_exposes_stat_types_not_restore_types() {
    let save_path = "brp_test_stat_schema.ron";
    let mut app = care_brp_app(save_path);

    let schema = brp_request(
        &mut app,
        "registry.schema",
        Some(serde_json::json!({
            "with_crates": ["alveus_types"]
        })),
    )
    .expect("registry.schema succeeds");
    let schema = schema.as_object().expect("schema map");

    for type_path in [
        "alveus_types::Stat",
        "alveus_types::FeedStat",
        "alveus_types::EnrichStat",
        "alveus_types::CleanStat",
    ] {
        assert!(schema.contains_key(type_path), "missing {type_path}");
    }

    for type_path in [
        "alveus_types::Restore",
        "alveus_types::FeedRestore",
        "alveus_types::EnrichRestore",
        "alveus_types::CleanRestore",
    ] {
        assert!(!schema.contains_key(type_path), "unexpected {type_path}");
    }

    let _ = std::fs::remove_file(save_path);
}
