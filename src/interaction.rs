use bevy::prelude::*;
use crate::components::{CurrentTilePosition, TilePosition};
use crate::content::{
    can_interact, item_display_name, InteractionKind, ItemId, RoomObjectId,
};
use crate::demo::player::Player;
use crate::screens::{InRoom, Screen};
use crate::stats::{AnimalId, ImproveStatEvent, StatTarget};

pub struct InteractionPlugin;

impl Plugin for InteractionPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<ItemId>()
            .register_type::<RoomObjectId>()
            .register_type::<Interactable>()
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
                    .run_if(in_interactive_room),
            )
            .add_systems(Update, handle_drop_input.run_if(on_overview_with_satchel_item));
    }
}

fn on_overview_with_satchel_item(screen: Res<State<Screen>>, satchel: Res<PlayerSatchel>) -> bool {
    matches!(screen.get(), Screen::Gameplay) && satchel.item.is_some()
}

#[derive(Component, Debug, Clone, Copy, Reflect)]
#[reflect(Component)]
pub struct Interactable {
    pub object_id: RoomObjectId,
    pub position: TilePosition,
    pub interaction: InteractionKind,
}

#[derive(Resource, Debug, Clone, Copy, Default, Reflect)]
#[reflect(Resource)]
pub struct PlayerSatchel {
    pub item: Option<ItemId>,
}

#[derive(Resource, Debug, Default)]
pub struct ActiveInteractionTarget {
    pub interactable: Option<Entity>,
}

#[derive(Resource, Debug, Default)]
pub struct LastPickupMessage {
    pub text: Option<String>,
    pub timer: Timer,
}

#[derive(Event, Debug, Clone, Copy)]
pub struct AnimalFedEvent {
    pub animal: AnimalId,
    pub dish_position: TilePosition,
}

fn in_interactive_room(screen: Res<State<Screen>>) -> bool {
    matches!(
        screen.get(),
        Screen::InRoom(InRoom::NutritionHouse) | Screen::InRoom(InRoom::PushPopEnclosure)
    )
}

fn update_interaction_target(
    player_query: Query<&CurrentTilePosition, With<Player>>,
    interactable_query: Query<(Entity, &Interactable)>,
    mut active: ResMut<ActiveInteractionTarget>,
) {
    let Ok(player_pos) = player_query.single() else {
        active.interactable = None;
        return;
    };

    active.interactable = interactable_query
        .iter()
        .find(|(_, interactable)| can_interact(player_pos.0, interactable.position))
        .map(|(entity, _)| entity);
}

fn handle_drop_input(
    input: Res<ButtonInput<KeyCode>>,
    mut satchel: ResMut<PlayerSatchel>,
    mut commands: Commands,
) {
    if !input.just_pressed(KeyCode::KeyK) {
        return;
    }

    let Some(item) = satchel.item else {
        return;
    };

    if let Err(message) = try_drop_item(&mut satchel) {
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
    interactable_query: Query<&Interactable>,
    mut satchel: ResMut<PlayerSatchel>,
    mut commands: Commands,
) {
    if !input.just_pressed(KeyCode::Space) {
        return;
    }

    let Some(entity) = active.interactable else {
        return;
    };

    let Ok(interactable) = interactable_query.get(entity) else {
        return;
    };

    match interactable.interaction {
        InteractionKind::GiveItem { item_id, .. } => {
            if let Err(message) = try_give_item(&mut satchel, item_id) {
                commands.insert_resource(LastPickupMessage {
                    text: Some(message.to_string()),
                    timer: Timer::from_seconds(2.5, TimerMode::Once),
                });
            } else {
                commands.insert_resource(LastPickupMessage {
                    text: Some(format!("Picked up {}", item_display_name(item_id))),
                    timer: Timer::from_seconds(2.5, TimerMode::Once),
                });
            }
        }
        InteractionKind::FeedAnimal {
            animal_id,
            required_item,
            stat,
            delta,
            ..
        } => {
            if let Err(message) = try_feed_animal(&mut satchel, required_item) {
                commands.insert_resource(LastPickupMessage {
                    text: Some(message.to_string()),
                    timer: Timer::from_seconds(2.5, TimerMode::Once),
                });
            } else {
                commands.trigger(ImproveStatEvent {
                    target: StatTarget::Animal {
                        id: animal_id,
                        stat,
                    },
                    amount: delta,
                });
                commands.trigger(AnimalFedEvent {
                    animal: animal_id,
                    dish_position: interactable.position,
                });
                commands.insert_resource(LastPickupMessage {
                    text: Some(format!("Fed {}", animal_display_name(animal_id))),
                    timer: Timer::from_seconds(2.5, TimerMode::Once),
                });
            }
        }
    }
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

pub fn try_feed_animal(
    satchel: &mut PlayerSatchel,
    required_item: ItemId,
) -> Result<(), &'static str> {
    match satchel.item {
        Some(item) if item == required_item => {
            satchel.item = None;
            Ok(())
        }
        Some(_) => Err("Wrong item for this feeding dish"),
        None => Err("You are not carrying any food"),
    }
}
