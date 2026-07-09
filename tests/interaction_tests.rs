use alveus_components::TilePosition;
use alveus_content::{ItemId, can_interact};
use alveus_interaction::{PlayerSatchel, try_drop_item, try_feed_animal, try_give_item};

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
    assert_eq!(satchel.item, Some(ItemId::TortoiseLeafyGreens));
}

#[test]
fn test_try_give_item_rejects_when_full() {
    let mut satchel = PlayerSatchel {
        item: Some(ItemId::ChickenGrains),
    };
    assert!(try_give_item(&mut satchel, ItemId::TortoiseLeafyGreens).is_err());
    assert_eq!(satchel.item, Some(ItemId::ChickenGrains));
}

#[test]
fn test_try_feed_animal_consumes_matching_item() {
    let mut satchel = PlayerSatchel {
        item: Some(ItemId::TortoiseLeafyGreens),
    };
    assert!(try_feed_animal(&mut satchel, ItemId::TortoiseLeafyGreens).is_ok());
    assert!(satchel.item.is_none());
}

#[test]
fn test_try_feed_animal_rejects_wrong_item() {
    let mut satchel = PlayerSatchel {
        item: Some(ItemId::ChickenGrains),
    };
    assert!(try_feed_animal(&mut satchel, ItemId::TortoiseLeafyGreens).is_err());
    assert_eq!(satchel.item, Some(ItemId::ChickenGrains));
}

#[test]
fn test_try_feed_animal_rejects_empty_satchel() {
    let mut satchel = PlayerSatchel::default();
    assert!(try_feed_animal(&mut satchel, ItemId::TortoiseLeafyGreens).is_err());
}

#[test]
fn test_try_drop_item() {
    let mut satchel = PlayerSatchel {
        item: Some(ItemId::ChickenGrains),
    };
    let dropped = try_drop_item(&mut satchel).unwrap();
    assert_eq!(dropped, ItemId::ChickenGrains);
    assert!(satchel.item.is_none());
    assert!(try_drop_item(&mut satchel).is_err());
}
