use crate::cleaning::{
    PoopDump, PoopDumpedEvent, PoopPile, PoopPickedUpEvent, PoopWheelbarrow, try_dump_poop,
    try_pickup_poop,
};
use crate::components::{CurrentTilePosition, TilePosition};
use crate::content::{ItemId, can_interact, item_display_name};
use crate::demo::player::Player;
use crate::screens::Screen;
use crate::stats::{AnimalId, AnimalStat, ImproveStatEvent, StatTarget};
use bevy::prelude::*;

pub struct InteractionPlugin;

impl Plugin for InteractionPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<ItemId>()
            .register_type::<Interactable>()
            .register_type::<GiveItem>()
            .register_type::<FeedAnimal>()
            .init_resource::<PlayerSatchel>()
            .init_resource::<ActiveInteractionTarget>()
            .init_resource::<LastPickupMessage>()
            .add_systems(
                Update,
                (
                    update_interaction_target,
                    handle_drop_input,
                    handle_interaction_input,
                    decay_pickup_message,
                )
                    .chain()
                    .run_if(allows_tile_interaction),
            )
            .add_systems(
                Update,
                handle_drop_input.run_if(on_overview_with_satchel_item),
            )
            .add_observer(apply_animal_fed);
    }
}

fn on_overview_with_satchel_item(screen: Res<State<Screen>>, satchel: Res<PlayerSatchel>) -> bool {
    matches!(screen.get(), Screen::Gameplay) && satchel.item.is_some()
}

/// Marker for tiles the player can interact with when adjacent.
#[derive(Component, Debug, Clone, Copy, Reflect, Default)]
#[reflect(Component, Default)]
pub struct Interactable;

/// Gives the player an item when interacted with.
///
/// An entity must not also have [`FeedAnimal`]. Bevy does not yet enforce
/// mutually exclusive components ([bevy#23569](https://github.com/bevyengine/bevy/issues/23569)).
/// Until that lands, authoring should ensure only one interaction component per tile.
#[derive(Component, Debug, Clone, Reflect)]
#[reflect(Component)]
#[require(Interactable)]
pub struct GiveItem {
    pub item_id: ItemId,
    pub prompt: String,
}

/// Feeds an animal when the player interacts with the correct item.
///
/// An entity must not also have [`GiveItem`]. Bevy does not yet enforce
/// mutually exclusive components ([bevy#23569](https://github.com/bevyengine/bevy/issues/23569)).
/// Until that lands, authoring should ensure only one interaction component per tile.
#[derive(Component, Debug, Clone, Reflect)]
#[reflect(Component)]
#[require(Interactable)]
pub struct FeedAnimal {
    pub animal_id: AnimalId,
    pub required_item: ItemId,
    pub stat: AnimalStat,
    pub delta: u32,
    pub prompt: String,
}

#[derive(Resource, Debug, Clone, Copy, Default, Reflect)]
#[reflect(Resource)]
pub struct PlayerSatchel {
    pub item: Option<ItemId>,
}

#[derive(Resource, Debug, Default, Reflect)]
#[reflect(Resource)]
pub struct ActiveInteractionTarget {
    pub interactable: Option<Entity>,
}

#[derive(Resource, Debug, Default, Reflect)]
#[reflect(Resource)]
pub struct LastPickupMessage {
    pub text: Option<String>,
    pub timer: Timer,
}

/// Emitted when the player successfully feeds an animal at a dish.
/// Item consumption, stat changes, and UI feedback are handled by observers.
#[derive(Event, Debug, Clone, Copy, Reflect)]
#[reflect(Event)]
pub struct AnimalFedEvent {
    pub animal_id: AnimalId,
    pub required_item: ItemId,
    pub stat: AnimalStat,
    pub delta: u32,
    pub dish_position: TilePosition,
}

fn allows_tile_interaction(screen: Res<State<Screen>>) -> bool {
    matches!(screen.get(), Screen::Gameplay | Screen::InRoom(_))
}

fn update_interaction_target(
    player_query: Query<&CurrentTilePosition, With<Player>>,
    interactable_query: Query<(Entity, &TilePosition), With<Interactable>>,
    mut active: ResMut<ActiveInteractionTarget>,
) {
    let Ok(player_pos) = player_query.single() else {
        active.interactable = None;
        return;
    };

    active.interactable = interactable_query
        .iter()
        .find(|(_, tile_pos)| can_interact(player_pos.0, **tile_pos))
        .map(|(entity, _)| entity);
}

fn handle_drop_input(
    input: Res<ButtonInput<KeyCode>>,
    mut satchel: ResMut<PlayerSatchel>,
    mut commands: Commands,
) {
    if input.just_pressed(KeyCode::KeyK) {
        perform_drop(&mut satchel, &mut commands);
    }
}

pub fn perform_drop(satchel: &mut PlayerSatchel, commands: &mut Commands) {
    let Some(item) = satchel.item else {
        return;
    };

    if let Err(message) = try_drop_item(satchel) {
        commands.insert_resource(LastPickupMessage {
            text: Some(message.to_string()),
            timer: Timer::from_seconds(2.5, TimerMode::Once),
        });
        return;
    }

    commands.insert_resource(LastPickupMessage {
        text: Some(format!("Dropped {}", item_display_name(item))),
        timer: Timer::from_seconds(2.5, TimerMode::Once),
    });
}

pub fn perform_drop_in_world(world: &mut World) {
    let drop_result = {
        let mut satchel = world.resource_mut::<PlayerSatchel>();
        let Some(item) = satchel.item else {
            return;
        };
        if let Err(message) = try_drop_item(&mut satchel) {
            Err(message.to_string())
        } else {
            Ok(item)
        }
    };

    let mut commands = world.commands();
    match drop_result {
        Ok(item) => {
            commands.insert_resource(LastPickupMessage {
                text: Some(format!("Dropped {}", item_display_name(item))),
                timer: Timer::from_seconds(2.5, TimerMode::Once),
            });
        }
        Err(message) => {
            commands.insert_resource(LastPickupMessage {
                text: Some(message),
                timer: Timer::from_seconds(2.5, TimerMode::Once),
            });
        }
    }
}

fn decay_pickup_message(time: Res<Time>, mut message: ResMut<LastPickupMessage>) {
    if message.text.is_none() {
        return;
    }
    message.timer.tick(time.delta());
    if message.timer.is_finished() {
        message.text = None;
    }
}

fn handle_interaction_input(
    input: Res<ButtonInput<KeyCode>>,
    active: Res<ActiveInteractionTarget>,
    tile_query: Query<&TilePosition>,
    give_query: Query<&GiveItem>,
    feed_query: Query<&FeedAnimal>,
    poop_query: Query<&PoopPile>,
    dump_query: Query<&PoopDump>,
    mut satchel: ResMut<PlayerSatchel>,
    mut wheelbarrow: ResMut<PoopWheelbarrow>,
    mut commands: Commands,
) {
    if input.just_pressed(KeyCode::Space) {
        perform_interact(
            &active,
            &tile_query,
            &give_query,
            &feed_query,
            &poop_query,
            &dump_query,
            &mut satchel,
            &mut wheelbarrow,
            &mut commands,
        );
    }
}

pub fn perform_interact(
    active: &ActiveInteractionTarget,
    tile_query: &Query<&TilePosition>,
    give_query: &Query<&GiveItem>,
    feed_query: &Query<&FeedAnimal>,
    poop_query: &Query<&PoopPile>,
    dump_query: &Query<&PoopDump>,
    satchel: &mut PlayerSatchel,
    wheelbarrow: &mut PoopWheelbarrow,
    commands: &mut Commands,
) {
    let Some(entity) = active.interactable else {
        return;
    };

    if let Ok(give) = give_query.get(entity) {
        if let Err(message) = try_give_item(satchel, give.item_id) {
            commands.insert_resource(LastPickupMessage {
                text: Some(message.to_string()),
                timer: Timer::from_seconds(2.5, TimerMode::Once),
            });
        } else {
            commands.insert_resource(LastPickupMessage {
                text: Some(format!("Picked up {}", item_display_name(give.item_id))),
                timer: Timer::from_seconds(2.5, TimerMode::Once),
            });
        }
        return;
    }

    if let Ok(feed) = feed_query.get(entity) {
        if let Err(message) = validate_feed_animal(satchel, feed.required_item) {
            commands.insert_resource(LastPickupMessage {
                text: Some(message.to_string()),
                timer: Timer::from_seconds(2.5, TimerMode::Once),
            });
            return;
        }

        let Ok(tile_pos) = tile_query.get(entity) else {
            return;
        };

        commands.trigger(AnimalFedEvent {
            animal_id: feed.animal_id,
            required_item: feed.required_item,
            stat: feed.stat,
            delta: feed.delta,
            dish_position: *tile_pos,
        });
        return;
    }

    if let Ok(poop) = poop_query.get(entity) {
        if let Err(message) = try_pickup_poop(wheelbarrow, poop.enclosure_id) {
            commands.insert_resource(LastPickupMessage {
                text: Some(message.to_string()),
                timer: Timer::from_seconds(2.5, TimerMode::Once),
            });
            return;
        }

        let Ok(tile_pos) = tile_query.get(entity) else {
            let _ = wheelbarrow.poops.pop();
            return;
        };

        commands.trigger(PoopPickedUpEvent {
            entity,
            enclosure_id: poop.enclosure_id,
            tile: *tile_pos,
        });
        return;
    }

    if let Ok(_dump) = dump_query.get(entity) {
        match try_dump_poop(wheelbarrow) {
            Ok(poops) => {
                commands.trigger(PoopDumpedEvent { poops });
            }
            Err(message) => {
                commands.insert_resource(LastPickupMessage {
                    text: Some(message.to_string()),
                    timer: Timer::from_seconds(2.5, TimerMode::Once),
                });
            }
        }
    }
}

pub fn perform_interact_in_world(world: &mut World) {
    let entity = match world.resource::<ActiveInteractionTarget>().interactable {
        Some(entity) => entity,
        None => return,
    };

    if let Some(give) = world.get::<GiveItem>(entity).cloned() {
        let pickup_message = {
            let mut satchel = world.resource_mut::<PlayerSatchel>();
            match try_give_item(&mut satchel, give.item_id) {
                Ok(()) => format!("Picked up {}", item_display_name(give.item_id)),
                Err(message) => message.to_string(),
            }
        };
        world.commands().insert_resource(LastPickupMessage {
            text: Some(pickup_message),
            timer: Timer::from_seconds(2.5, TimerMode::Once),
        });
        return;
    }

    if let Some(feed) = world.get::<FeedAnimal>(entity).cloned() {
        let satchel = world.resource::<PlayerSatchel>();
        if let Err(message) = validate_feed_animal(satchel, feed.required_item) {
            world.commands().insert_resource(LastPickupMessage {
                text: Some(message.to_string()),
                timer: Timer::from_seconds(2.5, TimerMode::Once),
            });
            return;
        }

        let Some(tile_pos) = world.get::<TilePosition>(entity).cloned() else {
            return;
        };

        world.trigger(AnimalFedEvent {
            animal_id: feed.animal_id,
            required_item: feed.required_item,
            stat: feed.stat,
            delta: feed.delta,
            dish_position: tile_pos,
        });
        return;
    }

    if let Some(poop) = world.get::<PoopPile>(entity).copied() {
        let pickup_result = {
            let mut wheelbarrow = world.resource_mut::<PoopWheelbarrow>();
            try_pickup_poop(&mut wheelbarrow, poop.enclosure_id)
        };
        if let Err(message) = pickup_result {
            world.commands().insert_resource(LastPickupMessage {
                text: Some(message.to_string()),
                timer: Timer::from_seconds(2.5, TimerMode::Once),
            });
            return;
        }

        let Some(tile_pos) = world.get::<TilePosition>(entity).cloned() else {
            let mut wheelbarrow = world.resource_mut::<PoopWheelbarrow>();
            let _ = wheelbarrow.poops.pop();
            return;
        };

        world.trigger(PoopPickedUpEvent {
            entity,
            enclosure_id: poop.enclosure_id,
            tile: tile_pos,
        });
        return;
    }

    if world.get::<PoopDump>(entity).is_some() {
        let dump_result = {
            let wheelbarrow = world.resource::<PoopWheelbarrow>();
            try_dump_poop(&wheelbarrow)
        };
        match dump_result {
            Ok(poops) => {
                world.trigger(PoopDumpedEvent { poops });
            }
            Err(message) => {
                world.commands().insert_resource(LastPickupMessage {
                    text: Some(message.to_string()),
                    timer: Timer::from_seconds(2.5, TimerMode::Once),
                });
            }
        }
    }
}

fn apply_animal_fed(
    trigger: On<AnimalFedEvent>,
    mut satchel: ResMut<PlayerSatchel>,
    mut commands: Commands,
) {
    let event = trigger.event();
    if let Err(message) = try_feed_animal(&mut satchel, event.required_item) {
        commands.insert_resource(LastPickupMessage {
            text: Some(message.to_string()),
            timer: Timer::from_seconds(2.5, TimerMode::Once),
        });
        return;
    }

    commands.trigger(ImproveStatEvent {
        target: StatTarget::Animal {
            id: event.animal_id,
            stat: event.stat,
        },
        amount: event.delta,
    });
    commands.insert_resource(LastPickupMessage {
        text: Some(format!("Fed {}", animal_display_name(event.animal_id))),
        timer: Timer::from_seconds(2.5, TimerMode::Once),
    });
}

fn animal_display_name(animal_id: AnimalId) -> &'static str {
    match animal_id {
        AnimalId::PushPop => "Push Pop",
        AnimalId::Polly => "Polly",
        AnimalId::Stompy => "Stompy",
        AnimalId::Georgie => "Georgie",
        AnimalId::Siren => "Siren",
    }
}

pub fn try_drop_item(satchel: &mut PlayerSatchel) -> Result<ItemId, &'static str> {
    satchel.item.take().ok_or("Satchel is already empty")
}

pub fn try_give_item(satchel: &mut PlayerSatchel, item_id: ItemId) -> Result<(), &'static str> {
    if satchel.item.is_some() {
        return Err("Satchel is full");
    }
    satchel.item = Some(item_id);
    Ok(())
}

pub fn validate_feed_animal(
    satchel: &PlayerSatchel,
    required_item: ItemId,
) -> Result<(), &'static str> {
    match satchel.item {
        Some(item) if item == required_item => Ok(()),
        Some(_) => Err("Wrong item for this feeding dish"),
        None => Err("You are not carrying any food"),
    }
}

pub fn try_feed_animal(
    satchel: &mut PlayerSatchel,
    required_item: ItemId,
) -> Result<(), &'static str> {
    validate_feed_animal(satchel, required_item)?;
    satchel.item = None;
    Ok(())
}
