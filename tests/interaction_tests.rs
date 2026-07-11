use alveus_components::TilePosition;
use alveus_configs::{SATCHEL_MAX_SLOTS, care_menu_options};
use alveus_content::{ItemId, can_interact};
use alveus_interaction::{
    CareMenuState, PlayerSatchel, care_menu_move_cursor, satchel_contains, try_drop_item,
    try_enrich_animal, try_feed_animal, try_give_item, try_take_item,
};
use alveus_types::{CareMenuId, ChoreId};

#[test]
fn test_can_interact_on_same_tile_and_adjacent() {
    let object = TilePosition { x: 5, y: 5 };

    assert!(can_interact(object, object));
    assert!(can_interact(TilePosition { x: 4, y: 5 }, object));
    assert!(can_interact(TilePosition { x: 6, y: 5 }, object));
    assert!(can_interact(TilePosition { x: 5, y: 4 }, object));
    assert!(can_interact(TilePosition { x: 5, y: 6 }, object));
    assert!(!can_interact(TilePosition { x: 7, y: 5 }, object));
}

#[test]
fn test_try_give_item_fills_empty_satchel() {
    let mut satchel = PlayerSatchel::default();
    assert!(try_give_item(&mut satchel, ItemId::TortoiseLeafyGreens).is_ok());
    assert_eq!(satchel.slots[0], Some(ItemId::TortoiseLeafyGreens));
    assert_eq!(satchel.slots[1], None);
}

#[test]
fn test_try_give_item_fills_second_slot() {
    let mut satchel = PlayerSatchel::default();
    try_give_item(&mut satchel, ItemId::ChickenGrains).unwrap();
    try_give_item(&mut satchel, ItemId::TortoiseLeafyGreens).unwrap();
    assert_eq!(satchel.slots[0], Some(ItemId::ChickenGrains));
    assert_eq!(satchel.slots[1], Some(ItemId::TortoiseLeafyGreens));
    assert_eq!(satchel.occupied_count(), SATCHEL_MAX_SLOTS);
}

#[test]
fn test_try_give_item_rejects_when_full() {
    let mut satchel = PlayerSatchel {
        slots: [Some(ItemId::ChickenGrains), Some(ItemId::MiniMirror)],
    };
    assert!(try_give_item(&mut satchel, ItemId::TortoiseLeafyGreens).is_err());
    assert_eq!(satchel.slots[0], Some(ItemId::ChickenGrains));
    assert_eq!(satchel.slots[1], Some(ItemId::MiniMirror));
}

#[test]
fn test_try_take_item_from_second_slot() {
    let mut satchel = PlayerSatchel {
        slots: [
            Some(ItemId::ChickenGrains),
            Some(ItemId::TortoiseLeafyGreens),
        ],
    };
    assert!(try_take_item(&mut satchel, ItemId::TortoiseLeafyGreens).is_ok());
    assert_eq!(satchel.slots[0], Some(ItemId::ChickenGrains));
    assert_eq!(satchel.slots[1], None);
}

#[test]
fn test_try_feed_animal_consumes_matching_item() {
    let mut satchel = PlayerSatchel::default();
    try_give_item(&mut satchel, ItemId::TortoiseLeafyGreens).unwrap();
    assert!(try_feed_animal(&mut satchel, ItemId::TortoiseLeafyGreens).is_ok());
    assert!(satchel.is_empty());
}

#[test]
fn test_try_feed_animal_rejects_wrong_item() {
    let mut satchel = PlayerSatchel::default();
    try_give_item(&mut satchel, ItemId::ChickenGrains).unwrap();
    assert!(try_feed_animal(&mut satchel, ItemId::TortoiseLeafyGreens).is_err());
    assert!(satchel_contains(&satchel, ItemId::ChickenGrains));
}

#[test]
fn test_try_feed_animal_rejects_empty_satchel() {
    let mut satchel = PlayerSatchel::default();
    assert!(try_feed_animal(&mut satchel, ItemId::TortoiseLeafyGreens).is_err());
}

#[test]
fn test_try_drop_item_drops_first_occupied() {
    let mut satchel = PlayerSatchel {
        slots: [Some(ItemId::ChickenGrains), Some(ItemId::MiniMirror)],
    };
    let dropped = try_drop_item(&mut satchel).unwrap();
    assert_eq!(dropped, ItemId::ChickenGrains);
    assert_eq!(satchel.slots[0], None);
    assert_eq!(satchel.slots[1], Some(ItemId::MiniMirror));
    assert!(try_drop_item(&mut satchel).is_ok());
    assert!(satchel.is_empty());
    assert!(try_drop_item(&mut satchel).is_err());
}

#[test]
fn test_try_enrich_animal_optional_item() {
    let mut satchel = PlayerSatchel::default();
    assert!(try_enrich_animal(&mut satchel, None).is_ok());
    try_give_item(&mut satchel, ItemId::MiniMirror).unwrap();
    assert!(try_enrich_animal(&mut satchel, Some(ItemId::MiniMirror)).is_ok());
    assert!(satchel.is_empty());
    assert!(try_enrich_animal(&mut satchel, Some(ItemId::MiniMirror)).is_err());
}

#[test]
fn test_mini_chore_satchel_transform() {
    let mut satchel = PlayerSatchel::default();
    try_give_item(&mut satchel, ItemId::RawVeggieTub).unwrap();

    // Pure satchel transform for a one-shot prep chore.
    assert_eq!(ChoreId::ChopVeggies.as_str(), "chop_veggies");
    try_take_item(&mut satchel, ItemId::RawVeggieTub).unwrap();
    try_give_item(&mut satchel, ItemId::PreparedVeggieDiet).unwrap();
    assert!(satchel_contains(&satchel, ItemId::PreparedVeggieDiet));
}

#[test]
fn test_open_menu_confirm_and_cancel() {
    let mut care_menu = CareMenuState::default();
    care_menu.menu_id = Some(CareMenuId::Fridge);
    care_menu.options = care_menu_options(CareMenuId::Fridge).to_vec();
    care_menu.cursor = 0;

    assert_eq!(
        care_menu.options.as_slice(),
        care_menu_options(CareMenuId::Fridge)
    );

    care_menu_move_cursor(&mut care_menu, 1);
    assert_eq!(care_menu.cursor, 1);
    care_menu_move_cursor(&mut care_menu, 1);
    assert_eq!(care_menu.cursor, 0);

    let mut satchel = PlayerSatchel::default();
    let item = care_menu.options[care_menu.cursor];
    try_give_item(&mut satchel, item).unwrap();
    assert!(satchel_contains(&satchel, ItemId::RawVeggieTub));

    care_menu = CareMenuState::default();
    assert!(care_menu.menu_id.is_none());
}
