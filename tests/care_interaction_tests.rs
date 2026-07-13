use alveus_app::{Menu, Screen};
use alveus_cleaning::CleaningPlugin;
use alveus_command::{CommandPlugin, GameCommand};
use alveus_components::{
    CareFeedbackEvent, CareHudPulse, CurrentTilePosition, Interactable, LastPickupMessage,
    MovementController, MovementIntent, Player, TilePosition,
};
use alveus_configs::{CARE_CLEAN_RESTORE, CARE_ENRICH_RESTORE, CARE_FEED_RESTORE};
use alveus_content::ItemId;
use alveus_hud::satchel_slots_label;
use alveus_input::InputPlugin;
use alveus_interaction::{
    ActiveInteractionTarget, AnimalCleanedEvent, AnimalEnrichedEvent, AnimalFedEvent,
    CareMenuState, CleanAnimal, EMPTY_CARE_MENU_MESSAGE, EnrichAnimal, GiveItem, InteractionPlugin,
    MISSING_CARE_MENU_MESSAGE, MiniChore, OpenMenu, PlayerSatchel, care_outcome_message,
    try_give_item,
};
use alveus_stats::{
    AnimalId, AnimalStat, AnimalStats, EnclosureId, EnclosureStats, SavePath, StatsPlugin,
};
use alveus_types::{CareMenuId, ChoreId, CleanStat, EnrichStat, FeedStat, Stat};
use bevy::prelude::*;
use bevy::state::app::StatesPlugin;
use bevy::time::TimeUpdateStrategy;

#[derive(Resource, Default)]
struct CapturedCareFeedback(Option<String>);

fn capture_care_feedback(
    trigger: On<CareFeedbackEvent>,
    mut captured: ResMut<CapturedCareFeedback>,
) {
    captured.0 = Some(trigger.event().message.clone());
}

fn care_test_app(save_path: &str) -> App {
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
        CleaningPlugin,
        InteractionPlugin,
        CommandPlugin,
        InputPlugin,
    ));
    app.add_observer(capture_care_feedback);
    app.world_mut()
        .spawn((Player, CurrentTilePosition(TilePosition { x: 0, y: 0 })));
    app.insert_resource(NextState::Pending(Screen::Gameplay));
    app.update();
    app
}

#[test]
fn enrich_animal_restores_happiness_without_item() {
    let save_path = "test_care_enrich.ron";
    let mut app = care_test_app(save_path);

    let push_pop = app
        .world_mut()
        .query_filtered::<Entity, With<AnimalId>>()
        .iter(app.world())
        .find(|&e| *app.world().get::<AnimalId>(e).unwrap() == AnimalId::PushPop)
        .expect("Push Pop");

    {
        let mut stats = app.world_mut().get_mut::<AnimalStats>(push_pop).unwrap();
        stats.happiness = Stat(100);
        stats.hunger = Stat(400);
    }
    let enc_entity = app
        .world_mut()
        .query_filtered::<Entity, With<EnclosureId>>()
        .iter(app.world())
        .find(|&e| *app.world().get::<EnclosureId>(e).unwrap() == EnclosureId::PushPopEnclosure)
        .expect("Push Pop enclosure");
    {
        let mut stats = app
            .world_mut()
            .get_mut::<EnclosureStats>(enc_entity)
            .unwrap();
        stats.cleanliness = Stat(500);
    }

    app.world_mut().trigger(AnimalEnrichedEvent {
        animal_id: AnimalId::PushPop,
        required_item: None,
        delta: CARE_ENRICH_RESTORE,
        station_position: TilePosition { x: 1, y: 1 },
    });
    app.update();

    let stats = app.world().get::<AnimalStats>(push_pop).unwrap();
    assert_eq!(stats.happiness, CARE_ENRICH_RESTORE.0);
    assert_eq!(stats.hunger, Stat(400));
    assert_eq!(
        app.world()
            .get::<EnclosureStats>(enc_entity)
            .unwrap()
            .cleanliness,
        Stat(500)
    );
    assert!(app.world().resource::<PlayerSatchel>().is_empty());

    let _ = std::fs::remove_file(save_path);
}

#[test]
fn enrich_animal_consumes_required_item() {
    let save_path = "test_care_enrich_item.ron";
    let mut app = care_test_app(save_path);

    let mut satchel = PlayerSatchel::default();
    try_give_item(&mut satchel, ItemId::MiniMirror).unwrap();
    app.insert_resource(satchel);

    let push_pop = app
        .world_mut()
        .query_filtered::<Entity, With<AnimalId>>()
        .iter(app.world())
        .find(|&e| *app.world().get::<AnimalId>(e).unwrap() == AnimalId::PushPop)
        .expect("Push Pop");

    {
        let mut stats = app.world_mut().get_mut::<AnimalStats>(push_pop).unwrap();
        stats.happiness = Stat(0);
    }

    app.world_mut().trigger(AnimalEnrichedEvent {
        animal_id: AnimalId::PushPop,
        required_item: Some(ItemId::MiniMirror),
        delta: CARE_ENRICH_RESTORE,
        station_position: TilePosition { x: 1, y: 1 },
    });
    app.update();

    assert!(app.world().resource::<PlayerSatchel>().is_empty());
    let stats = app.world().get::<AnimalStats>(push_pop).unwrap();
    assert_eq!(stats.happiness, CARE_ENRICH_RESTORE.0);

    let _ = std::fs::remove_file(save_path);
}

#[test]
fn interact_enrich_via_game_command() {
    let save_path = "test_care_enrich_cmd.ron";
    let mut app = care_test_app(save_path);

    let station = app
        .world_mut()
        .spawn((
            EnrichAnimal {
                animal_id: AnimalId::PushPop,
                required_item: None,
                delta: CARE_ENRICH_RESTORE,
                prompt: "Scatter hay".to_string(),
            },
            TilePosition { x: 0, y: 0 },
            Interactable,
        ))
        .id();

    app.world_mut()
        .resource_mut::<ActiveInteractionTarget>()
        .interactable = Some(station);

    let push_pop = app
        .world_mut()
        .query_filtered::<Entity, With<AnimalId>>()
        .iter(app.world())
        .find(|&e| *app.world().get::<AnimalId>(e).unwrap() == AnimalId::PushPop)
        .expect("Push Pop");
    {
        let mut stats = app.world_mut().get_mut::<AnimalStats>(push_pop).unwrap();
        stats.happiness = Stat(50);
    }

    app.world_mut().trigger(GameCommand::Interact);
    app.update();

    let stats = app.world().get::<AnimalStats>(push_pop).unwrap();
    assert_eq!(stats.happiness, CARE_ENRICH_RESTORE.0);

    let _ = std::fs::remove_file(save_path);
}

#[test]
fn mini_chore_completes_on_single_interact() {
    let save_path = "test_care_chore.ron";
    let mut app = care_test_app(save_path);

    let mut satchel = PlayerSatchel::default();
    try_give_item(&mut satchel, ItemId::RawVeggieTub).unwrap();
    app.insert_resource(satchel);

    let table = app
        .world_mut()
        .spawn((
            MiniChore {
                chore_id: ChoreId::ChopVeggies,
                required_item: Some(ItemId::RawVeggieTub),
                output_item: Some(ItemId::PreparedVeggieDiet),
                prompt: "Chop veggies".to_string(),
            },
            TilePosition { x: 0, y: 0 },
            Interactable,
        ))
        .id();

    app.world_mut()
        .resource_mut::<ActiveInteractionTarget>()
        .interactable = Some(table);

    app.world_mut().trigger(GameCommand::Interact);
    app.update();

    let satchel = app.world().resource::<PlayerSatchel>();
    assert!(satchel.slots.contains(&Some(ItemId::PreparedVeggieDiet)));
    assert!(!satchel.slots.contains(&Some(ItemId::RawVeggieTub)));

    let _ = std::fs::remove_file(save_path);
}

#[test]
fn open_menu_confirm_via_game_command() {
    let save_path = "test_care_menu.ron";
    let mut app = care_test_app(save_path);

    let fridge = app
        .world_mut()
        .spawn((
            OpenMenu {
                menu_id: CareMenuId::Fridge,
                prompt: "Open fridge".to_string(),
            },
            TilePosition { x: 0, y: 0 },
            Interactable,
        ))
        .id();

    app.world_mut()
        .resource_mut::<ActiveInteractionTarget>()
        .interactable = Some(fridge);

    app.world_mut().trigger(GameCommand::Interact);
    app.update();

    assert_eq!(
        *app.world().resource::<State<Menu>>().get(),
        Menu::CareItemPicker
    );
    assert_eq!(
        app.world().resource::<CareMenuState>().menu_id,
        Some(CareMenuId::Fridge)
    );

    app.world_mut().trigger(GameCommand::Continue);
    app.update();

    assert_eq!(*app.world().resource::<State<Menu>>().get(), Menu::None);
    let satchel = app.world().resource::<PlayerSatchel>();
    assert_eq!(satchel.slots[0], Some(ItemId::RawVeggieTub));

    let _ = std::fs::remove_file(save_path);
}

#[test]
fn open_menu_back_cancels() {
    let save_path = "test_care_menu_back.ron";
    let mut app = care_test_app(save_path);

    let fridge = app
        .world_mut()
        .spawn((
            OpenMenu {
                menu_id: CareMenuId::Fridge,
                prompt: "Open fridge".to_string(),
            },
            TilePosition { x: 0, y: 0 },
            Interactable,
        ))
        .id();

    app.world_mut()
        .resource_mut::<ActiveInteractionTarget>()
        .interactable = Some(fridge);

    app.world_mut().trigger(GameCommand::Interact);
    app.update();
    app.world_mut().trigger(GameCommand::Back);
    app.update();

    assert_eq!(*app.world().resource::<State<Menu>>().get(), Menu::None);
    assert!(app.world().resource::<PlayerSatchel>().is_empty());

    let _ = std::fs::remove_file(save_path);
}

#[test]
fn keyboard_picker_navigation_moves_once_per_press() {
    let save_path = "test_care_menu_keyboard.ron";
    let mut app = care_test_app(save_path);

    {
        let mut care_menu = app.world_mut().resource_mut::<CareMenuState>();
        care_menu.menu_id = Some(CareMenuId::Fridge);
        care_menu.options = alveus_configs::care_menu_options(CareMenuId::Fridge).to_vec();
        care_menu.cursor = 0;
    }
    app.world_mut()
        .resource_mut::<NextState<Menu>>()
        .set(Menu::CareItemPicker);
    app.update();

    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(KeyCode::KeyS);
    app.update();

    assert_eq!(app.world().resource::<CareMenuState>().cursor, 1);

    let _ = std::fs::remove_file(save_path);
}

#[test]
fn opening_picker_clears_world_interaction_state_but_preserves_picker_state() {
    let save_path = "test_care_menu_lifecycle.ron";
    let mut app = care_test_app(save_path);

    let player = app
        .world_mut()
        .query_filtered::<Entity, With<Player>>()
        .single(app.world())
        .expect("player");
    app.world_mut()
        .entity_mut(player)
        .insert(MovementController {
            intent: Some(MovementIntent::Right),
        });

    let fridge = app
        .world_mut()
        .spawn((
            OpenMenu {
                menu_id: CareMenuId::Fridge,
                prompt: "Open fridge".to_string(),
            },
            TilePosition { x: 0, y: 0 },
            Interactable,
        ))
        .id();
    app.world_mut()
        .resource_mut::<ActiveInteractionTarget>()
        .interactable = Some(fridge);

    app.world_mut().trigger(GameCommand::Interact);
    app.update();

    assert_eq!(
        *app.world().resource::<State<Menu>>().get(),
        Menu::CareItemPicker
    );
    assert!(
        app.world()
            .resource::<ActiveInteractionTarget>()
            .interactable
            .is_none()
    );
    assert_eq!(
        app.world()
            .get::<MovementController>(player)
            .unwrap()
            .intent,
        None
    );
    assert_eq!(
        app.world().resource::<CareMenuState>().menu_id,
        Some(CareMenuId::Fridge)
    );

    let _ = std::fs::remove_file(save_path);
}

#[test]
fn overlay_blocks_world_commands_and_closing_it_restores_target_discovery() {
    let save_path = "test_overlay_blocks_world_interaction.ron";
    let mut app = care_test_app(save_path);

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

    app.world_mut().trigger(GameCommand::Interact);
    app.world_mut().trigger(GameCommand::DropItem);
    app.update();

    assert_eq!(
        app.world().resource::<PlayerSatchel>().slots[0],
        Some(ItemId::RawVeggieTub)
    );
    assert!(
        !app.world()
            .resource::<PlayerSatchel>()
            .slots
            .contains(&Some(ItemId::MiniMirror))
    );

    app.world_mut()
        .resource_mut::<NextState<Menu>>()
        .set(Menu::None);
    app.update();

    assert_eq!(
        app.world()
            .resource::<ActiveInteractionTarget>()
            .interactable,
        Some(pickup)
    );

    let _ = std::fs::remove_file(save_path);
}

#[test]
fn world_commands_require_a_live_player_entity() {
    let save_path = "test_world_commands_require_player.ron";
    let mut app = care_test_app(save_path);

    let player = app
        .world_mut()
        .query_filtered::<Entity, With<Player>>()
        .single(app.world())
        .expect("player");
    app.world_mut().despawn(player);

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
        .resource_mut::<ActiveInteractionTarget>()
        .interactable = Some(pickup);

    app.world_mut().trigger(GameCommand::Interact);
    app.world_mut().trigger(GameCommand::DropItem);
    app.update();

    let satchel = app.world().resource::<PlayerSatchel>();
    assert_eq!(satchel.slots[0], Some(ItemId::RawVeggieTub));
    assert!(!satchel.slots.contains(&Some(ItemId::MiniMirror)));
    assert!(
        app.world()
            .resource::<ActiveInteractionTarget>()
            .interactable
            .is_none()
    );

    let _ = std::fs::remove_file(save_path);
}

#[test]
fn overlay_does_not_freeze_transient_feedback_timers() {
    let save_path = "test_overlay_feedback_timers.ron";
    let mut app = care_test_app(save_path);
    app.insert_resource(TimeUpdateStrategy::ManualDuration(
        std::time::Duration::from_secs(1),
    ));
    app.insert_resource(LastPickupMessage {
        text: Some("Temporary message".to_string()),
        timer: Timer::from_seconds(0.1, TimerMode::Once),
    });
    app.world_mut().resource_mut::<CareHudPulse>().trigger();
    app.world_mut()
        .resource_mut::<NextState<Menu>>()
        .set(Menu::Pause);

    app.update();
    app.update();

    assert!(app.world().resource::<LastPickupMessage>().text.is_none());
    assert!(!app.world().resource::<CareHudPulse>().is_active());

    let _ = std::fs::remove_file(save_path);
}

#[test]
fn keyboard_space_confirms_care_menu_via_game_command() {
    let save_path = "test_care_menu_space.ron";
    let mut app = care_test_app(save_path);

    {
        let mut care_menu = app.world_mut().resource_mut::<CareMenuState>();
        care_menu.menu_id = Some(CareMenuId::Fridge);
        care_menu.options = alveus_configs::care_menu_options(CareMenuId::Fridge).to_vec();
        care_menu.cursor = 0;
    }
    app.world_mut()
        .resource_mut::<NextState<Menu>>()
        .set(Menu::CareItemPicker);
    app.update();

    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(KeyCode::Space);
    app.update();

    assert_eq!(*app.world().resource::<State<Menu>>().get(), Menu::None);
    assert_eq!(
        app.world().resource::<PlayerSatchel>().slots[0],
        Some(ItemId::RawVeggieTub)
    );
    assert!(!app.world().resource::<CareHudPulse>().is_active());

    let _ = std::fs::remove_file(save_path);
}

#[test]
fn keyboard_escape_cancels_care_menu_via_game_command() {
    let save_path = "test_care_menu_escape.ron";
    let mut app = care_test_app(save_path);

    {
        let mut care_menu = app.world_mut().resource_mut::<CareMenuState>();
        care_menu.menu_id = Some(CareMenuId::Fridge);
        care_menu.options = alveus_configs::care_menu_options(CareMenuId::Fridge).to_vec();
        care_menu.cursor = 0;
    }
    app.world_mut()
        .resource_mut::<NextState<Menu>>()
        .set(Menu::CareItemPicker);
    app.update();

    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(KeyCode::Escape);
    app.update();

    assert_eq!(*app.world().resource::<State<Menu>>().get(), Menu::None);
    assert!(app.world().resource::<CareMenuState>().menu_id.is_none());
    assert!(app.world().resource::<PlayerSatchel>().is_empty());

    let _ = std::fs::remove_file(save_path);
}

#[test]
fn picker_left_and_right_are_noops_and_interact_confirms() {
    let save_path = "test_care_menu_direction_parity.ron";
    let mut app = care_test_app(save_path);

    {
        let mut care_menu = app.world_mut().resource_mut::<CareMenuState>();
        care_menu.menu_id = Some(CareMenuId::Fridge);
        care_menu.options = alveus_configs::care_menu_options(CareMenuId::Fridge).to_vec();
        care_menu.cursor = 1;
    }
    app.world_mut()
        .resource_mut::<NextState<Menu>>()
        .set(Menu::CareItemPicker);
    app.update();

    app.world_mut()
        .trigger(GameCommand::Move(MovementIntent::Left));
    app.world_mut()
        .trigger(GameCommand::Move(MovementIntent::Right));
    app.update();
    assert_eq!(app.world().resource::<CareMenuState>().cursor, 1);

    app.world_mut().trigger(GameCommand::Interact);
    app.update();
    assert_eq!(*app.world().resource::<State<Menu>>().get(), Menu::None);
    assert_eq!(
        app.world().resource::<PlayerSatchel>().slots[0],
        Some(ItemId::TortoiseLeafyGreens)
    );

    let _ = std::fs::remove_file(save_path);
}

#[test]
fn empty_picker_confirms_by_cancelling_with_explicit_copy() {
    let save_path = "test_care_menu_empty.ron";
    let mut app = care_test_app(save_path);
    app.insert_resource(CareMenuState {
        menu_id: Some(CareMenuId::Fridge),
        options: Vec::new(),
        cursor: 0,
    });
    app.world_mut()
        .resource_mut::<NextState<Menu>>()
        .set(Menu::CareItemPicker);
    app.update();

    app.world_mut().trigger(GameCommand::Continue);
    app.update();

    assert_eq!(*app.world().resource::<State<Menu>>().get(), Menu::None);
    assert_eq!(
        app.world().resource::<LastPickupMessage>().text.as_deref(),
        Some(EMPTY_CARE_MENU_MESSAGE)
    );
    assert!(app.world().resource::<CareMenuState>().menu_id.is_none());

    let _ = std::fs::remove_file(save_path);
}

#[test]
fn missing_picker_id_confirms_by_cancelling_with_explicit_copy() {
    let save_path = "test_care_menu_missing_id.ron";
    let mut app = care_test_app(save_path);
    app.insert_resource(CareMenuState {
        menu_id: None,
        options: vec![ItemId::RawVeggieTub],
        cursor: 0,
    });
    app.world_mut()
        .resource_mut::<NextState<Menu>>()
        .set(Menu::CareItemPicker);
    app.update();

    app.world_mut().trigger(GameCommand::Continue);
    app.update();

    assert_eq!(*app.world().resource::<State<Menu>>().get(), Menu::None);
    assert_eq!(
        app.world().resource::<LastPickupMessage>().text.as_deref(),
        Some(MISSING_CARE_MENU_MESSAGE)
    );
    assert!(app.world().resource::<PlayerSatchel>().is_empty());

    let _ = std::fs::remove_file(save_path);
}

#[test]
fn picker_inventory_edges_follow_satchel_capacity_rules() {
    let save_path = "test_care_menu_inventory_edges.ron";
    let mut app = care_test_app(save_path);

    app.insert_resource(PlayerSatchel {
        slots: [Some(ItemId::RawVeggieTub), None],
    });
    app.insert_resource(CareMenuState {
        menu_id: Some(CareMenuId::Fridge),
        options: vec![ItemId::RawVeggieTub],
        cursor: 0,
    });
    app.world_mut()
        .resource_mut::<NextState<Menu>>()
        .set(Menu::CareItemPicker);
    app.update();
    app.world_mut().trigger(GameCommand::Continue);
    app.update();
    assert_eq!(
        app.world().resource::<PlayerSatchel>().slots,
        [Some(ItemId::RawVeggieTub), Some(ItemId::RawVeggieTub)]
    );

    app.insert_resource(CareMenuState {
        menu_id: Some(CareMenuId::Fridge),
        options: vec![ItemId::TortoiseLeafyGreens],
        cursor: 0,
    });
    app.world_mut()
        .resource_mut::<NextState<Menu>>()
        .set(Menu::CareItemPicker);
    app.update();
    app.world_mut().trigger(GameCommand::Continue);
    app.update();
    assert_eq!(
        app.world().resource::<PlayerSatchel>().slots,
        [Some(ItemId::RawVeggieTub), Some(ItemId::RawVeggieTub)]
    );
    assert_eq!(
        app.world().resource::<LastPickupMessage>().text.as_deref(),
        Some("Satchel is full")
    );

    let _ = std::fs::remove_file(save_path);
}

#[test]
fn care_hud_pulse_fires_on_stat_restore_not_pickup() {
    let save_path = "test_care_hud_pulse.ron";
    let mut app = care_test_app(save_path);

    let station = app
        .world_mut()
        .spawn((
            EnrichAnimal {
                animal_id: AnimalId::PushPop,
                required_item: None,
                delta: CARE_ENRICH_RESTORE,
                prompt: "Scatter hay".to_string(),
            },
            TilePosition { x: 0, y: 0 },
            Interactable,
        ))
        .id();
    app.world_mut()
        .resource_mut::<ActiveInteractionTarget>()
        .interactable = Some(station);

    let push_pop = app
        .world_mut()
        .query_filtered::<Entity, With<AnimalId>>()
        .iter(app.world())
        .find(|&e| *app.world().get::<AnimalId>(e).unwrap() == AnimalId::PushPop)
        .expect("Push Pop");
    {
        let mut stats = app.world_mut().get_mut::<AnimalStats>(push_pop).unwrap();
        stats.happiness = Stat(50);
    }

    app.world_mut().trigger(GameCommand::Interact);
    app.update();

    assert!(app.world().resource::<CareHudPulse>().is_active());

    // Reset pulse, then pick up an item — toast only, no restore pulse.
    *app.world_mut().resource_mut::<CareHudPulse>() = CareHudPulse::default();
    let pile = app
        .world_mut()
        .spawn((
            GiveItem {
                item_id: ItemId::ChickenGrains,
                prompt: "Take grains".to_string(),
            },
            TilePosition { x: 0, y: 0 },
            Interactable,
        ))
        .id();
    app.world_mut()
        .resource_mut::<ActiveInteractionTarget>()
        .interactable = Some(pile);

    app.world_mut().trigger(GameCommand::Interact);
    app.update();

    assert_eq!(
        app.world().resource::<PlayerSatchel>().slots[0],
        Some(ItemId::ChickenGrains)
    );
    assert!(!app.world().resource::<CareHudPulse>().is_active());
    assert_eq!(
        app.world().resource::<CapturedCareFeedback>().0.as_deref(),
        Some("Picked up Chicken Grains")
    );

    let _ = std::fs::remove_file(save_path);
}

#[test]
fn drop_item_via_game_command() {
    let save_path = "test_care_drop.ron";
    let mut app = care_test_app(save_path);

    let mut satchel = PlayerSatchel::default();
    try_give_item(&mut satchel, ItemId::ChickenGrains).unwrap();
    try_give_item(&mut satchel, ItemId::MiniMirror).unwrap();
    app.insert_resource(satchel);

    app.world_mut().trigger(GameCommand::DropItem);
    app.update();

    let satchel = app.world().resource::<PlayerSatchel>();
    assert_eq!(satchel.slots[0], None);
    assert_eq!(satchel.slots[1], Some(ItemId::MiniMirror));

    let _ = std::fs::remove_file(save_path);
}

#[test]
fn care_feedback_event_is_registered() {
    let mut app = App::new();
    alveus_reflect::register_types(&mut app);
    let registry = app.world().resource::<AppTypeRegistry>();
    let registry = registry.read();
    assert!(
        registry
            .get(std::any::TypeId::of::<CareFeedbackEvent>())
            .is_some()
    );
    assert!(
        registry
            .get(std::any::TypeId::of::<EnrichAnimal>())
            .is_some()
    );
    assert!(
        registry
            .get(std::any::TypeId::of::<CleanAnimal>())
            .is_some()
    );
    assert!(
        registry
            .get(std::any::TypeId::of::<AnimalCleanedEvent>())
            .is_some()
    );
    assert!(registry.get(std::any::TypeId::of::<Stat>()).is_some());
    assert!(registry.get(std::any::TypeId::of::<FeedStat>()).is_some());
    assert!(registry.get(std::any::TypeId::of::<EnrichStat>()).is_some());
    assert!(registry.get(std::any::TypeId::of::<CleanStat>()).is_some());
    assert!(registry.get(std::any::TypeId::of::<MiniChore>()).is_some());
    assert!(registry.get(std::any::TypeId::of::<OpenMenu>()).is_some());
}

#[test]
fn enrich_happiness_feedback_says_enriched() {
    let save_path = "test_care_feedback_enrich.ron";
    let mut app = care_test_app(save_path);

    app.world_mut().trigger(AnimalEnrichedEvent {
        animal_id: AnimalId::PushPop,
        required_item: None,
        delta: CARE_ENRICH_RESTORE,
        station_position: TilePosition { x: 1, y: 1 },
    });
    app.update();

    let text = app
        .world()
        .resource::<CapturedCareFeedback>()
        .0
        .as_deref()
        .expect("care feedback");
    assert!(text.contains("Enriched"), "{text}");
    assert!(!text.contains("Cleaned"), "{text}");
    assert_eq!(
        care_outcome_message(AnimalId::PushPop, AnimalStat::Happiness),
        text
    );
    assert!(app.world().resource::<LastPickupMessage>().text.is_none());

    let _ = std::fs::remove_file(save_path);
}

#[test]
fn clean_animal_restores_cleanliness_only() {
    let save_path = "test_care_feedback_clean.ron";
    let mut app = care_test_app(save_path);

    let push_pop = app
        .world_mut()
        .query_filtered::<Entity, With<AnimalId>>()
        .iter(app.world())
        .find(|&e| *app.world().get::<AnimalId>(e).unwrap() == AnimalId::PushPop)
        .expect("Push Pop");
    {
        let mut stats = app.world_mut().get_mut::<AnimalStats>(push_pop).unwrap();
        stats.hunger = Stat(300);
        stats.happiness = Stat(400);
    }

    let enc_entity = app
        .world_mut()
        .query_filtered::<Entity, With<EnclosureId>>()
        .iter(app.world())
        .find(|&e| *app.world().get::<EnclosureId>(e).unwrap() == EnclosureId::PushPopEnclosure)
        .expect("Push Pop enclosure");
    {
        let mut stats = app
            .world_mut()
            .get_mut::<EnclosureStats>(enc_entity)
            .unwrap();
        stats.cleanliness = Stat(100);
    }

    app.world_mut().trigger(AnimalCleanedEvent {
        animal_id: AnimalId::PushPop,
        required_item: None,
        delta: CARE_CLEAN_RESTORE,
        station_position: TilePosition { x: 1, y: 1 },
    });
    app.update();

    let text = app
        .world()
        .resource::<CapturedCareFeedback>()
        .0
        .as_deref()
        .expect("care feedback");
    assert!(text.contains("Cleaned"), "{text}");
    assert!(!text.contains("Enriched"), "{text}");
    assert_eq!(
        care_outcome_message(AnimalId::PushPop, AnimalStat::Cleanliness),
        text
    );
    assert!(app.world().resource::<LastPickupMessage>().text.is_none());

    let enc_stats = app.world().get::<EnclosureStats>(enc_entity).unwrap();
    assert_eq!(enc_stats.cleanliness, CARE_CLEAN_RESTORE.0);
    let stats = app.world().get::<AnimalStats>(push_pop).unwrap();
    assert_eq!(stats.hunger, Stat(300));
    assert_eq!(stats.happiness, Stat(400));

    let _ = std::fs::remove_file(save_path);
}

#[test]
fn interact_clean_via_game_command() {
    let save_path = "test_care_clean_cmd.ron";
    let mut app = care_test_app(save_path);

    let station = app
        .world_mut()
        .spawn((
            CleanAnimal {
                animal_id: AnimalId::PushPop,
                required_item: None,
                delta: CARE_CLEAN_RESTORE,
                prompt: "Sweep nesting".to_string(),
            },
            TilePosition { x: 0, y: 0 },
            Interactable,
        ))
        .id();

    app.world_mut()
        .resource_mut::<ActiveInteractionTarget>()
        .interactable = Some(station);

    let enc_entity = app
        .world_mut()
        .query_filtered::<Entity, With<EnclosureId>>()
        .iter(app.world())
        .find(|&e| *app.world().get::<EnclosureId>(e).unwrap() == EnclosureId::PushPopEnclosure)
        .expect("Push Pop enclosure");
    {
        let mut stats = app
            .world_mut()
            .get_mut::<EnclosureStats>(enc_entity)
            .unwrap();
        stats.cleanliness = Stat(50);
    }

    app.world_mut().trigger(GameCommand::Interact);
    app.update();

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
    assert!(text.contains("Cleaned"), "{text}");
    assert!(!text.contains("Enriched"), "{text}");
    assert!(app.world().resource::<LastPickupMessage>().text.is_none());

    let _ = std::fs::remove_file(save_path);
}

#[test]
fn feed_feedback_says_fed() {
    let save_path = "test_care_feedback_feed.ron";
    let mut app = care_test_app(save_path);

    let mut satchel = PlayerSatchel::default();
    try_give_item(&mut satchel, ItemId::TortoiseLeafyGreens).unwrap();
    app.insert_resource(satchel);

    let push_pop = app
        .world_mut()
        .query_filtered::<Entity, With<AnimalId>>()
        .iter(app.world())
        .find(|&e| *app.world().get::<AnimalId>(e).unwrap() == AnimalId::PushPop)
        .expect("Push Pop");
    {
        let mut stats = app.world_mut().get_mut::<AnimalStats>(push_pop).unwrap();
        stats.hunger = Stat(100);
        stats.happiness = Stat(200);
    }
    let enc_entity = app
        .world_mut()
        .query_filtered::<Entity, With<EnclosureId>>()
        .iter(app.world())
        .find(|&e| *app.world().get::<EnclosureId>(e).unwrap() == EnclosureId::PushPopEnclosure)
        .expect("Push Pop enclosure");
    {
        let mut stats = app
            .world_mut()
            .get_mut::<EnclosureStats>(enc_entity)
            .unwrap();
        stats.cleanliness = Stat(300);
    }

    app.world_mut().trigger(AnimalFedEvent {
        animal_id: AnimalId::PushPop,
        required_item: ItemId::TortoiseLeafyGreens,
        delta: CARE_FEED_RESTORE,
        dish_position: TilePosition { x: 8, y: 6 },
    });
    app.update();

    let text = app
        .world()
        .resource::<CapturedCareFeedback>()
        .0
        .as_deref()
        .expect("care feedback");
    assert!(text.contains("Fed"), "{text}");
    assert!(!text.contains("Cleaned"), "{text}");
    assert!(!text.contains("Enriched"), "{text}");
    assert_eq!(
        care_outcome_message(AnimalId::PushPop, AnimalStat::Hunger),
        text
    );
    assert!(app.world().resource::<LastPickupMessage>().text.is_none());

    let stats = app.world().get::<AnimalStats>(push_pop).unwrap();
    assert_eq!(stats.hunger, CARE_FEED_RESTORE.0);
    assert_eq!(stats.happiness, Stat(200));
    assert_eq!(
        app.world()
            .get::<EnclosureStats>(enc_entity)
            .unwrap()
            .cleanliness,
        Stat(300)
    );

    let _ = std::fs::remove_file(save_path);
}

#[test]
fn satchel_card_slots_remain_visible_after_care() {
    let save_path = "test_care_satchel_slots.ron";
    let mut app = care_test_app(save_path);

    let mut satchel = PlayerSatchel::default();
    try_give_item(&mut satchel, ItemId::TortoiseLeafyGreens).unwrap();
    try_give_item(&mut satchel, ItemId::MiniMirror).unwrap();
    app.insert_resource(satchel);

    app.world_mut().trigger(AnimalFedEvent {
        animal_id: AnimalId::PushPop,
        required_item: ItemId::TortoiseLeafyGreens,
        delta: CARE_FEED_RESTORE,
        dish_position: TilePosition { x: 8, y: 6 },
    });
    app.update();

    let satchel = *app.world().resource::<PlayerSatchel>();
    assert_eq!(satchel.slots[0], None);
    assert_eq!(satchel.slots[1], Some(ItemId::MiniMirror));

    let feedback = app
        .world()
        .resource::<CapturedCareFeedback>()
        .0
        .as_deref()
        .expect("care feedback toast");
    assert!(feedback.contains("Fed"), "{feedback}");
    assert!(app.world().resource::<LastPickupMessage>().text.is_none());

    let label = satchel_slots_label(&satchel);
    assert!(label.contains("Slot 1:"), "{label}");
    assert!(label.contains("Slot 2:"), "{label}");
    assert!(label.contains("Mini Mirror"), "{label}");
    assert!(!label.contains("Fed"), "{label}");

    let _ = std::fs::remove_file(save_path);
}
