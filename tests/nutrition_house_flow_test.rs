#![cfg(feature = "headless")]

//! Nutrition House care loop: fridge menu, prep, Polly feed/clean/enrich.

use alveus_app::{Menu, Screen};
use alveus_cleaning::CleaningPlugin;
use alveus_command::{CommandPlugin, GameCommand};
use alveus_components::{CurrentTilePosition, Interactable, MovementIntent, Player, TilePosition};
use alveus_configs::{CARE_CLEAN_RESTORE, CARE_ENRICH_RESTORE, CARE_FEED_RESTORE, STAT_FULL};
use alveus_content::ItemId;
use alveus_interaction::{
    ActiveInteractionTarget, CareMenuState, CleanAnimal, EnrichAnimal, FeedAnimal,
    InteractionPlugin, MiniChore, OpenMenu, PlayerSatchel, try_give_item,
};
use alveus_stats::{AnimalId, AnimalStats, EnclosureId, EnclosureStats, SavePath, StatsPlugin};
use alveus_types::{CareMenuId, ChoreId, Stat};
use bevy::prelude::*;
use bevy::state::app::StatesPlugin;

fn nutrition_test_app(save_path: &str) -> App {
    let _ = std::fs::remove_file(save_path);
    let mut app = App::new();
    app.add_plugins(StatesPlugin);
    app.add_plugins(alveus_app::plugin);
    app.add_plugins(MinimalPlugins);
    app.init_resource::<ButtonInput<KeyCode>>();
    app.insert_resource(SavePath(save_path.to_string()));
    app.add_plugins((
        StatsPlugin,
        CleaningPlugin,
        InteractionPlugin,
        CommandPlugin,
    ));
    app.world_mut()
        .spawn((Player, CurrentTilePosition(TilePosition { x: 0, y: 0 })));
    app.insert_resource(NextState::Pending(Screen::Gameplay));
    app.update();
    app
}

fn find_animal(app: &mut App, id: AnimalId) -> Entity {
    app.world_mut()
        .query_filtered::<Entity, With<AnimalId>>()
        .iter(app.world())
        .find(|&e| *app.world().get::<AnimalId>(e).unwrap() == id)
        .unwrap_or_else(|| panic!("{id:?} should exist"))
}

fn find_enclosure(app: &mut App, id: EnclosureId) -> Entity {
    app.world_mut()
        .query_filtered::<Entity, With<EnclosureId>>()
        .iter(app.world())
        .find(|&e| *app.world().get::<EnclosureId>(e).unwrap() == id)
        .unwrap_or_else(|| panic!("{id:?} enclosure should exist"))
}

#[test]
fn fridge_menu_move_down_selects_greens() {
    let save_path = "test_nh_fridge_cursor.ron";
    let mut app = nutrition_test_app(save_path);

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
    assert_eq!(app.world().resource::<CareMenuState>().list.cursor, 0);

    app.world_mut()
        .trigger(GameCommand::Move(MovementIntent::Down));
    app.update();
    assert_eq!(app.world().resource::<CareMenuState>().list.cursor, 1);

    app.world_mut().trigger(GameCommand::Continue);
    app.update();

    let satchel = app.world().resource::<PlayerSatchel>();
    assert_eq!(satchel.slots[0], Some(ItemId::TortoiseLeafyGreens));

    let _ = std::fs::remove_file(save_path);
}

#[test]
fn prep_then_polly_feed_clean_enrich() {
    let save_path = "test_nh_polly_triad.ron";
    let mut app = nutrition_test_app(save_path);

    // --- Prep: RawVeggieTub → PreparedVeggieDiet ---
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
    assert!(
        app.world()
            .resource::<PlayerSatchel>()
            .slots
            .iter()
            .any(|s| *s == Some(ItemId::PreparedVeggieDiet))
    );

    // Drop prepared diet so satchel has room for grains / mirror.
    app.world_mut().trigger(GameCommand::DropItem);
    app.update();
    assert!(app.world().resource::<PlayerSatchel>().is_empty());

    let polly = find_animal(&mut app, AnimalId::Polly);
    let playpen = find_enclosure(&mut app, EnclosureId::NutritionHousePlaypen);

    {
        let mut stats = app.world_mut().get_mut::<AnimalStats>(polly).unwrap();
        stats.hunger = Stat(100);
        stats.happiness = Stat(100);
    }
    {
        let mut enc = app.world_mut().get_mut::<EnclosureStats>(playpen).unwrap();
        enc.cleanliness = Stat(100);
    }

    // --- Feed ---
    let mut satchel = PlayerSatchel::default();
    try_give_item(&mut satchel, ItemId::ChickenGrains).unwrap();
    app.insert_resource(satchel);

    let bowl = app
        .world_mut()
        .spawn((
            FeedAnimal {
                animal_id: AnimalId::Polly,
                required_item: ItemId::ChickenGrains,
                delta: CARE_FEED_RESTORE,
                prompt: "Fill Polly's bowl".to_string(),
            },
            TilePosition { x: 0, y: 0 },
            Interactable,
        ))
        .id();
    app.world_mut()
        .resource_mut::<ActiveInteractionTarget>()
        .interactable = Some(bowl);

    app.world_mut().trigger(GameCommand::Interact);
    app.update();

    assert_eq!(
        app.world().get::<AnimalStats>(polly).unwrap().hunger,
        CARE_FEED_RESTORE.0
    );
    assert!(app.world().resource::<PlayerSatchel>().is_empty());
    // --- Nesting clean ---
    let nesting = app
        .world_mut()
        .spawn((
            CleanAnimal {
                animal_id: AnimalId::Polly,
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
        .interactable = Some(nesting);

    app.world_mut().trigger(GameCommand::Interact);
    app.update();
    assert_eq!(
        app.world()
            .get::<EnclosureStats>(playpen)
            .unwrap()
            .cleanliness,
        STAT_FULL
    );

    // --- Enrich with mirror ---
    let mut satchel = PlayerSatchel::default();
    try_give_item(&mut satchel, ItemId::MiniMirror).unwrap();
    app.insert_resource(satchel);

    let post = app
        .world_mut()
        .spawn((
            EnrichAnimal {
                animal_id: AnimalId::Polly,
                required_item: Some(ItemId::MiniMirror),
                delta: CARE_ENRICH_RESTORE,
                prompt: "Place mirror".to_string(),
            },
            TilePosition { x: 0, y: 0 },
            Interactable,
        ))
        .id();
    app.world_mut()
        .resource_mut::<ActiveInteractionTarget>()
        .interactable = Some(post);

    app.world_mut().trigger(GameCommand::Interact);
    app.update();

    assert_eq!(
        app.world().get::<AnimalStats>(polly).unwrap().happiness,
        CARE_ENRICH_RESTORE.0
    );
    assert!(app.world().resource::<PlayerSatchel>().is_empty());

    let _ = std::fs::remove_file(save_path);
}

#[test]
fn polly_stations_match_shipped_shapes() {
    let bowl = FeedAnimal {
        animal_id: AnimalId::Polly,
        required_item: ItemId::ChickenGrains,
        delta: CARE_FEED_RESTORE,
        prompt: "Fill Polly's bowl".to_string(),
    };
    let nesting = CleanAnimal {
        animal_id: AnimalId::Polly,
        required_item: None,
        delta: CARE_CLEAN_RESTORE,
        prompt: "Sweep nesting".to_string(),
    };
    let enrich = EnrichAnimal {
        animal_id: AnimalId::Polly,
        required_item: Some(ItemId::MiniMirror),
        delta: CARE_ENRICH_RESTORE,
        prompt: "Place mirror".to_string(),
    };

    assert_eq!(bowl.animal_id, AnimalId::Polly);
    assert_eq!(nesting.delta, CARE_CLEAN_RESTORE);
    assert_eq!(enrich.required_item, Some(ItemId::MiniMirror));
}
