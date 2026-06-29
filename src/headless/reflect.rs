//! Reflect registration for BRP introspection (`registry.schema`, `world.query`, etc.).

use bevy::prelude::*;

use crate::components::{
    BuildingEntrance, CurrentTilePosition, DesiredTilePosition, DynamicObstacle, InEnclosure,
    Obstacle, PersistedDynamicObstacle, TilePosition,
};
use crate::cleaning::{
    PoopDump, PoopDumpedEvent, PoopPickedUpEvent, PoopPile, PoopWheelbarrow,
};
use crate::content::{ItemId, RoomObjectId};
use crate::demo::movement::{MovementController, MovementIntent};
use crate::demo::player::Player;
use crate::demo::room::PlayerSpawnPoint;
use crate::demo::toast::{DismissToastEvent, TriggerToastEvent};
use crate::demo::entrance::{PlayerEnteredBuildingEvent, PlayerExitedBuildingEvent};
use crate::interaction::{
    ActiveInteractionTarget, AnimalFedEvent, FeedAnimal, GiveItem, Interactable, LastPickupMessage,
    PlayerSatchel,
};
use crate::menus::{main::PlayClickEvent, Menu};
use crate::screens::{InRoom, Screen};
use crate::stats::{
    AnimalEnclosure, AnimalId, AnimalName, AnimalStat, AnimalStats, AnimalTilePosition, EnclosureId,
    EnclosureName, EnclosureStats, ImproveStatEvent, SanctuaryUpkeep, SavePath, StatTarget,
    WorsenStatEvent,
};

use super::camera::HeadlessRenderTarget;
use super::command::{GameCommand, StepRequest};

pub fn register_headless_types(app: &mut App) {
    app.register_type::<Screen>()
        .register_type::<InRoom>()
        .register_type::<Menu>()
        .register_type::<crate::Pause>()
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
        .register_type::<PoopPile>()
        .register_type::<PoopDump>()
        .register_type::<PoopWheelbarrow>()
        .register_type::<RoomObjectId>()
        .register_type::<ItemId>()
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
        .register_type::<PlayerSatchel>()
        .register_type::<ActiveInteractionTarget>()
        .register_type::<LastPickupMessage>()
        .register_type::<PlayerSpawnPoint>()
        .register_type::<SanctuaryUpkeep>()
        .register_type::<SavePath>()
        .register_type::<HeadlessRenderTarget>()
        .register_type::<StepRequest>()
        .register_type::<GameCommand>()
        .register_type::<PlayClickEvent>()
        .register_type::<AnimalFedEvent>()
        .register_type::<PoopPickedUpEvent>()
        .register_type::<PoopDumpedEvent>()
        .register_type::<ImproveStatEvent>()
        .register_type::<WorsenStatEvent>()
        .register_type::<PlayerEnteredBuildingEvent>()
        .register_type::<PlayerExitedBuildingEvent>()
        .register_type::<TriggerToastEvent>()
        .register_type::<DismissToastEvent>();
}
