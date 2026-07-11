//! Reflect registration for BRP introspection (`registry.schema`, `world.query`, etc.).
//!
//! This is the single canonical registration entry point, reused by the game
//! binary, the headless server, and the `gen_tiled_types` exporter.

use bevy::prelude::*;

use alveus_app::{InRoom, Menu, Pause, Screen};
use alveus_cleaning::{PoopDump, PoopDumpedEvent, PoopPickedUpEvent, PoopPile};
use alveus_components::{
    BuildingEntrance, CareFeedbackEvent, CareHudPulse, CurrentTilePosition, DesiredTilePosition,
    DynamicObstacle, InEnclosure, Interactable, LastPickupMessage, Obstacle,
    PersistedDynamicObstacle, Player, PoopWheelbarrow, TilePosition,
};
use alveus_components::{MovementController, MovementIntent};
use alveus_content::{ItemId, RoomObjectId};
use alveus_interaction::{
    ActiveInteractionTarget, AnimalCleanedEvent, AnimalEnrichedEvent, AnimalFedEvent,
    CareMenuState, CleanAnimal, EnrichAnimal, FeedAnimal, GiveItem, MiniChore, OpenMenu,
    PlayerSatchel,
};
use alveus_menus::PlayClickEvent;
use alveus_stats::{
    AnimalEnclosure, AnimalId, AnimalName, AnimalStat, AnimalStats, AnimalTilePosition,
    EnclosureId, EnclosureName, EnclosureStats, ImproveStatEvent, SanctuaryUpkeep, SavePath,
    StatTarget, WorsenStatEvent,
};
use alveus_types::{CareMenuId, ChoreId, CleanStat, EnrichStat, FeedStat, Stat};
use alveus_world::entrance::{PlayerEnteredBuildingEvent, PlayerExitedBuildingEvent};
use alveus_world::room::PlayerSpawnPoint;
use alveus_world::toast::{DismissToastEvent, TriggerToastEvent};

use crate::camera::HeadlessRenderTarget;
use crate::command::{GameCommand, StepRequest};

pub fn register_headless_types(app: &mut App) {
    app.register_type::<Screen>()
        .register_type::<State<Screen>>()
        .register_type::<InRoom>()
        .register_type::<Menu>()
        .register_type::<State<Menu>>()
        .register_type::<Pause>()
        .register_type::<State<Pause>>()
        .register_type::<TilePosition>()
        .register_type::<CurrentTilePosition>()
        .register_type::<DesiredTilePosition>()
        .register_type::<BuildingEntrance>()
        .register_type::<Obstacle>()
        .register_type::<DynamicObstacle>()
        .register_type::<InEnclosure>()
        .register_type::<PersistedDynamicObstacle>()
        .register_type::<Player>()
        .register_type::<MovementController>()
        .register_type::<MovementIntent>()
        .register_type::<Interactable>()
        .register_type::<GiveItem>()
        .register_type::<FeedAnimal>()
        .register_type::<EnrichAnimal>()
        .register_type::<CleanAnimal>()
        .register_type::<MiniChore>()
        .register_type::<OpenMenu>()
        .register_type::<PoopPile>()
        .register_type::<PoopDump>()
        .register_type::<PoopWheelbarrow>()
        .register_type::<RoomObjectId>()
        .register_type::<ItemId>()
        .register_type::<ChoreId>()
        .register_type::<CareMenuId>()
        .register_type::<AnimalId>()
        .register_type::<AnimalName>()
        .register_type::<AnimalStats>()
        .register_type::<AnimalEnclosure>()
        .register_type::<AnimalTilePosition>()
        .register_type::<EnclosureId>()
        .register_type::<EnclosureName>()
        .register_type::<EnclosureStats>()
        .register_type::<AnimalStat>()
        .register_type::<StatTarget>()
        .register_type::<Stat>()
        .register_type::<FeedStat>()
        .register_type::<EnrichStat>()
        .register_type::<CleanStat>()
        .register_type::<PlayerSatchel>()
        .register_type::<ActiveInteractionTarget>()
        .register_type::<CareMenuState>()
        .register_type::<LastPickupMessage>()
        .register_type::<CareFeedbackEvent>()
        .register_type::<CareHudPulse>()
        .register_type::<PlayerSpawnPoint>()
        .register_type::<SanctuaryUpkeep>()
        .register_type::<SavePath>()
        .register_type::<HeadlessRenderTarget>()
        .register_type::<StepRequest>()
        .register_type::<GameCommand>()
        .register_type::<PlayClickEvent>()
        .register_type::<AnimalFedEvent>()
        .register_type::<AnimalEnrichedEvent>()
        .register_type::<AnimalCleanedEvent>()
        .register_type::<PoopPickedUpEvent>()
        .register_type::<PoopDumpedEvent>()
        .register_type::<ImproveStatEvent>()
        .register_type::<WorsenStatEvent>()
        .register_type::<PlayerEnteredBuildingEvent>()
        .register_type::<PlayerExitedBuildingEvent>()
        .register_type::<TriggerToastEvent>()
        .register_type::<DismissToastEvent>();
}
