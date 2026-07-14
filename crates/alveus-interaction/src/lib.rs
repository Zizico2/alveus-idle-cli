//! Player care interaction dispatch: give, feed, enrich, clean, mini-chore,
//! open menu, and cleaning hand-off (poop pickup/dump).

use alveus_app::{Menu, Screen, tile_interaction_enabled, tile_interaction_enabled_for};
use alveus_cleaning::{
    PoopDump, PoopDumpedEvent, PoopPickedUpEvent, PoopPile, PoopWheelbarrow, try_dump_poop,
    try_pickup_poop,
};
use alveus_components::{
    CareFeedbackEvent, CareHudPulse, CurrentTilePosition, Interactable, LastPickupMessage,
    MovementController, Player, TilePosition,
};
use alveus_configs::{care_menu_options, item_display_name, prep_recipe_for};
use alveus_content::{ItemId, can_interact};
use alveus_menus_models::{ListMenuDirection, ListMenuState};
use alveus_stats::{AnimalStat, ImproveStatEvent, StatTarget};
use alveus_types::{AnimalId, CareMenuId, ChoreId, CleanStat, EnrichStat, FeedStat};
use bevy::prelude::*;

// Compatibility re-export: the satchel remains available from the established
// interaction API even though its shared storage type lives below this crate.
pub use alveus_components::PlayerSatchel;

/// Care-specific context wrapped around the shared list-menu model.
/// `list.cursor` is authoritative for keyboard, pointer, and BRP interaction.
#[derive(Resource, Debug, Default, Clone, Reflect)]
#[reflect(Resource)]
pub struct CareMenuState {
    pub menu_id: Option<CareMenuId>,
    pub list: ListMenuState<ItemId>,
}

impl CareMenuState {
    pub fn new(menu_id: Option<CareMenuId>, options: impl IntoIterator<Item = ItemId>) -> Self {
        Self {
            menu_id,
            list: ListMenuState::new(options),
        }
    }
}

/// Internal command-routing events (not Reflect-registered).
#[doc(hidden)]
#[derive(Event, Debug, Clone, Copy)]
pub enum InteractionRequest {
    Interact,
    DropItem,
    NavigateCareMenu(ListMenuDirection),
    ConfirmCareMenu,
    CancelCareMenu,
}

/// Adds player care interactions.
///
/// Requires [`alveus_app::plugin`] to initialize the app-wide states first.
pub struct InteractionPlugin;

impl Plugin for InteractionPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<ItemId>()
            .register_type::<ChoreId>()
            .register_type::<CareMenuId>()
            .register_type::<Interactable>()
            .register_type::<GiveItem>()
            .register_type::<FeedAnimal>()
            .register_type::<EnrichAnimal>()
            .register_type::<CleanAnimal>()
            .register_type::<MiniChore>()
            .register_type::<OpenMenu>()
            .register_type::<PlayerSatchel>()
            .register_type::<ActiveInteractionTarget>()
            .register_type::<CareMenuState>()
            .register_type::<AnimalFedEvent>()
            .register_type::<AnimalEnrichedEvent>()
            .register_type::<AnimalCleanedEvent>()
            .register_type::<CareFeedbackEvent>()
            .register_type::<CareHudPulse>();
        app.init_resource::<PlayerSatchel>()
            .init_resource::<ActiveInteractionTarget>()
            .init_resource::<CareMenuState>()
            .init_resource::<LastPickupMessage>()
            .init_resource::<CareHudPulse>()
            .add_systems(OnExit(Menu::None), clear_world_input_state)
            .add_systems(
                Update,
                update_interaction_target.run_if(tile_interaction_enabled),
            )
            .add_systems(Update, (decay_pickup_message, tick_care_hud_pulse))
            .add_observer(apply_animal_fed)
            .add_observer(apply_animal_enriched)
            .add_observer(apply_animal_cleaned)
            .add_observer(on_interaction_interact)
            .add_observer(on_interaction_drop_item)
            .add_observer(on_interaction_navigate_care_menu)
            .add_observer(on_interaction_confirm_care_menu)
            .add_observer(on_interaction_cancel_care_menu);
    }
}

fn clear_world_input_state(
    mut active: ResMut<ActiveInteractionTarget>,
    mut players: Query<&mut MovementController, With<Player>>,
) {
    active.interactable = None;
    for mut movement in &mut players {
        movement.intent = None;
    }
}

/// Gives the player an item when interacted with.
///
/// An entity must not also have another care interaction component. Bevy does
/// not yet enforce mutually exclusive components
/// ([bevy#23569](https://github.com/bevyengine/bevy/issues/23569)). Until that
/// lands, authoring should ensure only one interaction component per tile.
#[derive(Component, Debug, Clone, Reflect)]
#[reflect(Component)]
#[require(Interactable)]
pub struct GiveItem {
    pub item_id: ItemId,
    pub prompt: String,
}

/// Feeds an animal when the player interacts with the correct item.
/// Always restores hunger; the amount is a [`FeedStat`].
#[derive(Component, Debug, Clone, Reflect)]
#[reflect(Component)]
#[require(Interactable)]
pub struct FeedAnimal {
    pub animal_id: AnimalId,
    pub required_item: ItemId,
    pub delta: FeedStat,
    pub prompt: String,
}

/// Enriches an animal (happiness). Optional required item.
/// Always restores happiness; the amount is an [`EnrichStat`].
#[derive(Component, Debug, Clone, Reflect)]
#[reflect(Component)]
#[require(Interactable)]
pub struct EnrichAnimal {
    pub animal_id: AnimalId,
    pub required_item: Option<ItemId>,
    pub delta: EnrichStat,
    pub prompt: String,
}

/// Cleans an animal's enclosure. Optional required item.
/// Always restores enclosure cleanliness; the amount is a [`CleanStat`].
#[derive(Component, Debug, Clone, Reflect)]
#[reflect(Component)]
#[require(Interactable)]
pub struct CleanAnimal {
    pub animal_id: AnimalId,
    pub required_item: Option<ItemId>,
    pub delta: CleanStat,
    pub prompt: String,
}

/// One-shot prep / transform chore (e.g. chop veggies at the prep table).
#[derive(Component, Debug, Clone, Reflect)]
#[reflect(Component)]
#[require(Interactable)]
pub struct MiniChore {
    pub chore_id: ChoreId,
    pub required_item: Option<ItemId>,
    pub output_item: Option<ItemId>,
    pub prompt: String,
}

/// Opens a care item-picker menu identified by [`CareMenuId`].
#[derive(Component, Debug, Clone, Reflect)]
#[reflect(Component)]
#[require(Interactable)]
pub struct OpenMenu {
    pub menu_id: CareMenuId,
    pub prompt: String,
}

#[derive(Resource, Debug, Default, Reflect)]
#[reflect(Resource)]
pub struct ActiveInteractionTarget {
    pub interactable: Option<Entity>,
}

/// Emitted when the player successfully feeds an animal at a dish.
#[derive(Event, Debug, Clone, Copy, Reflect)]
#[reflect(Event)]
pub struct AnimalFedEvent {
    pub animal_id: AnimalId,
    pub required_item: ItemId,
    pub delta: FeedStat,
    pub dish_position: TilePosition,
}

/// Emitted when the player successfully enriches an animal.
#[derive(Event, Debug, Clone, Copy, Reflect)]
#[reflect(Event)]
pub struct AnimalEnrichedEvent {
    pub animal_id: AnimalId,
    pub required_item: Option<ItemId>,
    pub delta: EnrichStat,
    pub station_position: TilePosition,
}

/// Emitted when the player successfully cleans an animal's enclosure.
#[derive(Event, Debug, Clone, Copy, Reflect)]
#[reflect(Event)]
pub struct AnimalCleanedEvent {
    pub animal_id: AnimalId,
    pub required_item: Option<ItemId>,
    pub delta: CleanStat,
    pub station_position: TilePosition,
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

pub fn perform_drop(satchel: &mut PlayerSatchel, commands: &mut Commands) {
    match try_drop_item(satchel) {
        Ok(item) => {
            set_pickup_message(commands, format!("Dropped {}", item_display_name(item)));
        }
        Err(_) => {
            // Empty satchel: no-op (matches GameCommand::DropItem docs).
        }
    }
}

fn on_interaction_drop_item(
    trigger: On<InteractionRequest>,
    screen: Res<State<Screen>>,
    menu: Res<State<Menu>>,
    mut satchel: ResMut<PlayerSatchel>,
    mut commands: Commands,
    players: Query<(), With<Player>>,
) {
    if !matches!(trigger.event(), InteractionRequest::DropItem) {
        return;
    }
    if !tile_interaction_enabled_for(*screen.get(), *menu.get()) || players.single().is_err() {
        return;
    }
    perform_drop(&mut satchel, &mut commands);
}

fn on_interaction_navigate_care_menu(
    trigger: On<InteractionRequest>,
    mut care_menu: ResMut<CareMenuState>,
    players: Query<(), With<Player>>,
    mut movement: Query<&mut MovementController, With<Player>>,
) {
    let InteractionRequest::NavigateCareMenu(direction) = *trigger.event() else {
        return;
    };
    if players.single().is_err() {
        return;
    }
    if let Ok(mut movement) = movement.single_mut() {
        movement.intent = None;
    }
    care_menu_move_cursor(&mut care_menu, direction.delta());
}

fn on_interaction_confirm_care_menu(
    trigger: On<InteractionRequest>,
    mut care_menu: ResMut<CareMenuState>,
    mut satchel: ResMut<PlayerSatchel>,
    mut next_menu: ResMut<NextState<Menu>>,
    mut commands: Commands,
) {
    if !matches!(trigger.event(), InteractionRequest::ConfirmCareMenu) {
        return;
    }
    confirm_care_menu(&mut care_menu, &mut satchel, &mut next_menu, &mut commands);
}

fn on_interaction_cancel_care_menu(
    trigger: On<InteractionRequest>,
    mut care_menu: ResMut<CareMenuState>,
    mut next_menu: ResMut<NextState<Menu>>,
) {
    if !matches!(trigger.event(), InteractionRequest::CancelCareMenu) {
        return;
    }
    cancel_care_menu(&mut care_menu, &mut next_menu);
}

fn on_interaction_interact(
    trigger: On<InteractionRequest>,
    screen: Res<State<Screen>>,
    menu: Res<State<Menu>>,
    active: Res<ActiveInteractionTarget>,
    mut satchel: ResMut<PlayerSatchel>,
    mut care_menu: ResMut<CareMenuState>,
    mut next_menu: ResMut<NextState<Menu>>,
    mut commands: Commands,
    players: Query<(), With<Player>>,
    wheelbarrow: Option<ResMut<PoopWheelbarrow>>,
    targets: Query<(
        &TilePosition,
        Option<&GiveItem>,
        Option<&FeedAnimal>,
        Option<&EnrichAnimal>,
        Option<&CleanAnimal>,
        Option<&MiniChore>,
        Option<&OpenMenu>,
        Option<&PoopPile>,
        Option<&PoopDump>,
    )>,
) {
    if !matches!(trigger.event(), InteractionRequest::Interact) {
        return;
    }
    if *menu.get() == Menu::CareItemPicker {
        if players.single().is_err() {
            return;
        }
        confirm_care_menu(&mut care_menu, &mut satchel, &mut next_menu, &mut commands);
        return;
    }
    if !tile_interaction_enabled_for(*screen.get(), *menu.get()) || players.single().is_err() {
        return;
    }
    let Some(entity) = active.interactable else {
        return;
    };
    let Ok((tile_pos, give, feed, enrich, clean, chore, open, poop, dump)) = targets.get(entity)
    else {
        return;
    };

    if let Some(give) = give {
        match try_give_item(&mut satchel, give.item_id) {
            Ok(()) => {
                commands.trigger(CareFeedbackEvent {
                    message: format!("Picked up {}", item_display_name(give.item_id)),
                });
            }
            Err(message) => set_pickup_message(&mut commands, message.to_string()),
        }
        return;
    }

    if let Some(feed) = feed {
        if let Err(message) = validate_has_item(&satchel, feed.required_item) {
            set_pickup_message(&mut commands, message.to_string());
            return;
        }
        commands.trigger(AnimalFedEvent {
            animal_id: feed.animal_id,
            required_item: feed.required_item,
            delta: feed.delta,
            dish_position: *tile_pos,
        });
        return;
    }

    if let Some(enrich) = enrich {
        if let Some(required) = enrich.required_item
            && let Err(message) = validate_has_item(&satchel, required)
        {
            set_pickup_message(&mut commands, message.to_string());
            return;
        }
        commands.trigger(AnimalEnrichedEvent {
            animal_id: enrich.animal_id,
            required_item: enrich.required_item,
            delta: enrich.delta,
            station_position: *tile_pos,
        });
        return;
    }

    if let Some(clean) = clean {
        if let Some(required) = clean.required_item
            && let Err(message) = validate_has_item(&satchel, required)
        {
            set_pickup_message(&mut commands, message.to_string());
            return;
        }
        commands.trigger(AnimalCleanedEvent {
            animal_id: clean.animal_id,
            required_item: clean.required_item,
            delta: clean.delta,
            station_position: *tile_pos,
        });
        return;
    }

    if let Some(chore) = chore {
        handle_mini_chore(chore, &mut satchel, &mut commands);
        return;
    }

    if let Some(open) = open {
        open_care_menu(open.menu_id, &mut care_menu, &mut next_menu);
        return;
    }

    if let Some(poop) = poop {
        let Some(mut wheelbarrow) = wheelbarrow else {
            return;
        };
        if let Err(message) = try_pickup_poop(&mut wheelbarrow, poop.enclosure_id) {
            set_pickup_message(&mut commands, message.to_string());
            return;
        }
        commands.trigger(PoopPickedUpEvent {
            entity,
            enclosure_id: poop.enclosure_id,
            tile: *tile_pos,
        });
        return;
    }

    if dump.is_some() {
        let Some(wheelbarrow) = wheelbarrow.as_deref() else {
            return;
        };
        match try_dump_poop(wheelbarrow) {
            Ok(poops) => commands.trigger(PoopDumpedEvent { poops }),
            Err(message) => set_pickup_message(&mut commands, message.to_string()),
        }
    }
}

fn handle_mini_chore(chore: &MiniChore, satchel: &mut PlayerSatchel, commands: &mut Commands) {
    if let Some(required) = chore.required_item
        && let Err(message) = validate_has_item(satchel, required)
    {
        set_pickup_message(commands, message.to_string());
        return;
    }

    if let Some(required_item) = chore.required_item {
        if let Err(message) = try_take_item(satchel, required_item) {
            set_pickup_message(commands, message.to_string());
            return;
        }
        if let Some(output) = chore
            .output_item
            .or_else(|| prep_recipe_for(chore.chore_id, required_item).map(|r| r.output))
        {
            if let Err(message) = try_give_item(satchel, output) {
                let _ = try_give_item(satchel, required_item);
                set_pickup_message(commands, message.to_string());
                return;
            }
            commands.trigger(CareFeedbackEvent {
                message: format!("Prepared {}", item_display_name(output)),
            });
            return;
        }
    } else if let Some(output) = chore.output_item {
        if let Err(message) = try_give_item(satchel, output) {
            set_pickup_message(commands, message.to_string());
            return;
        }
        commands.trigger(CareFeedbackEvent {
            message: format!("Got {}", item_display_name(output)),
        });
        return;
    }

    commands.trigger(CareFeedbackEvent {
        message: format!("Finished {}", chore.prompt),
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

fn tick_care_hud_pulse(time: Res<Time>, mut pulse: ResMut<CareHudPulse>) {
    if pulse.timer.duration().as_secs_f32() > 0.0 {
        pulse.timer.tick(time.delta());
    }
}

pub fn open_care_menu(
    menu_id: CareMenuId,
    care_menu: &mut CareMenuState,
    next_menu: &mut NextState<Menu>,
) {
    *care_menu = CareMenuState::new(Some(menu_id), care_menu_options(menu_id).iter().copied());
    next_menu.set(Menu::CareItemPicker);
}

pub fn care_menu_move_cursor(care_menu: &mut CareMenuState, delta: i32) {
    care_menu.list.move_cursor(delta);
}

/// Select an available care-menu row. UI hover/click adapters use this same
/// authoritative cursor as keyboard, gamepad, and BRP commands.
pub fn care_menu_set_cursor(care_menu: &mut CareMenuState, index: usize) -> bool {
    care_menu.list.set_cursor(index)
}

pub const EMPTY_CARE_MENU_MESSAGE: &str = "No items are available";
pub const MISSING_CARE_MENU_MESSAGE: &str = "This item menu is unavailable";

fn selected_care_menu_item(care_menu: &CareMenuState) -> Result<ItemId, &'static str> {
    if care_menu.menu_id.is_none() {
        return Err(MISSING_CARE_MENU_MESSAGE);
    }
    care_menu
        .list
        .selected()
        .copied()
        .ok_or(EMPTY_CARE_MENU_MESSAGE)
}

pub fn confirm_care_menu(
    care_menu: &mut CareMenuState,
    satchel: &mut PlayerSatchel,
    next_menu: &mut NextState<Menu>,
    commands: &mut Commands,
) {
    let item = match selected_care_menu_item(care_menu) {
        Ok(item) => item,
        Err(message) => {
            set_pickup_message(commands, message.to_string());
            cancel_care_menu(care_menu, next_menu);
            return;
        }
    };
    match try_give_item(satchel, item) {
        Ok(()) => {
            emit_care_feedback(commands, format!("Took {}", item_display_name(item)));
        }
        Err(message) => {
            set_pickup_message(commands, message.to_string());
        }
    }
    *care_menu = CareMenuState::default();
    next_menu.set(Menu::None);
}

pub fn cancel_care_menu(care_menu: &mut CareMenuState, next_menu: &mut NextState<Menu>) {
    *care_menu = CareMenuState::default();
    next_menu.set(Menu::None);
}

fn apply_animal_fed(
    trigger: On<AnimalFedEvent>,
    mut satchel: ResMut<PlayerSatchel>,
    mut pulse: ResMut<CareHudPulse>,
    mut commands: Commands,
) {
    let event = trigger.event();
    if let Err(message) = try_take_item(&mut satchel, event.required_item) {
        set_pickup_message(&mut commands, message.to_string());
        return;
    }

    commands.trigger(ImproveStatEvent {
        target: StatTarget::Animal {
            id: event.animal_id,
            stat: AnimalStat::Hunger,
        },
        amount: event.delta.into(),
    });
    // Outcome toast via CareFeedbackEvent; satchel card stays inventory-only.
    emit_care_feedback(
        &mut commands,
        care_outcome_message(event.animal_id, AnimalStat::Hunger),
    );
    pulse.trigger();
}

fn apply_animal_enriched(
    trigger: On<AnimalEnrichedEvent>,
    mut satchel: ResMut<PlayerSatchel>,
    mut pulse: ResMut<CareHudPulse>,
    mut commands: Commands,
) {
    let event = trigger.event();
    if let Some(required) = event.required_item
        && let Err(message) = try_take_item(&mut satchel, required)
    {
        set_pickup_message(&mut commands, message.to_string());
        return;
    }

    commands.trigger(ImproveStatEvent {
        target: StatTarget::Animal {
            id: event.animal_id,
            stat: AnimalStat::Happiness,
        },
        amount: event.delta.into(),
    });
    emit_care_feedback(
        &mut commands,
        care_outcome_message(event.animal_id, AnimalStat::Happiness),
    );
    pulse.trigger();
}

fn apply_animal_cleaned(
    trigger: On<AnimalCleanedEvent>,
    mut satchel: ResMut<PlayerSatchel>,
    mut pulse: ResMut<CareHudPulse>,
    mut commands: Commands,
) {
    let event = trigger.event();
    if let Some(required) = event.required_item
        && let Err(message) = try_take_item(&mut satchel, required)
    {
        set_pickup_message(&mut commands, message.to_string());
        return;
    }

    commands.trigger(ImproveStatEvent {
        target: StatTarget::Animal {
            id: event.animal_id,
            stat: AnimalStat::Cleanliness,
        },
        amount: event.delta.into(),
    });
    emit_care_feedback(
        &mut commands,
        care_outcome_message(event.animal_id, AnimalStat::Cleanliness),
    );
    pulse.trigger();
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

/// Player-facing care outcome copy keyed by the restored [`AnimalStat`].
pub fn care_outcome_message(animal_id: AnimalId, stat: AnimalStat) -> String {
    let name = animal_display_name(animal_id);
    match stat {
        AnimalStat::Cleanliness => format!("Cleaned {name}"),
        AnimalStat::Happiness => format!("Enriched {name}"),
        AnimalStat::Hunger => format!("Fed {name}"),
    }
}

fn set_pickup_message(commands: &mut Commands, text: String) {
    commands.insert_resource(LastPickupMessage {
        text: Some(text),
        timer: Timer::from_seconds(2.5, TimerMode::Once),
    });
}

fn emit_care_feedback(commands: &mut Commands, message: String) {
    commands.trigger(CareFeedbackEvent { message });
}

// --- Pure satchel helpers (unit-tested) ---

/// Drop the first occupied slot. Returns the dropped item.
pub fn try_drop_item(satchel: &mut PlayerSatchel) -> Result<ItemId, &'static str> {
    for slot in &mut satchel.slots {
        if let Some(item) = slot.take() {
            return Ok(item);
        }
    }
    Err("Satchel is already empty")
}

/// Place `item_id` in the first empty slot.
pub fn try_give_item(satchel: &mut PlayerSatchel, item_id: ItemId) -> Result<(), &'static str> {
    if satchel.is_full() {
        return Err("Satchel is full");
    }
    for slot in &mut satchel.slots {
        if slot.is_none() {
            *slot = Some(item_id);
            return Ok(());
        }
    }
    Err("Satchel is full")
}

pub fn satchel_contains(satchel: &PlayerSatchel, item_id: ItemId) -> bool {
    satchel.slots.contains(&Some(item_id))
}

/// Remove the first slot matching `item_id`.
pub fn try_take_item(satchel: &mut PlayerSatchel, item_id: ItemId) -> Result<(), &'static str> {
    for slot in &mut satchel.slots {
        if *slot == Some(item_id) {
            *slot = None;
            return Ok(());
        }
    }
    if satchel.is_empty() {
        Err("You are not carrying any food")
    } else {
        Err("Wrong item for this feeding dish")
    }
}

pub fn validate_has_item(
    satchel: &PlayerSatchel,
    required_item: ItemId,
) -> Result<(), &'static str> {
    if satchel_contains(satchel, required_item) {
        Ok(())
    } else if satchel.is_empty() {
        Err("You are not carrying any food")
    } else {
        Err("Wrong item for this feeding dish")
    }
}

/// Back-compat alias used by older tests / call sites.
pub fn validate_feed_animal(
    satchel: &PlayerSatchel,
    required_item: ItemId,
) -> Result<(), &'static str> {
    validate_has_item(satchel, required_item)
}

pub fn try_feed_animal(
    satchel: &mut PlayerSatchel,
    required_item: ItemId,
) -> Result<(), &'static str> {
    try_take_item(satchel, required_item)
}

pub fn try_enrich_animal(
    satchel: &mut PlayerSatchel,
    required_item: Option<ItemId>,
) -> Result<(), &'static str> {
    match required_item {
        Some(item) => try_take_item(satchel, item),
        None => Ok(()),
    }
}

#[cfg(test)]
mod care_outcome_tests {
    use super::*;

    #[test]
    fn care_outcome_message_matches_stat() {
        assert_eq!(
            care_outcome_message(AnimalId::PushPop, AnimalStat::Hunger),
            "Fed Push Pop"
        );
        assert_eq!(
            care_outcome_message(AnimalId::Polly, AnimalStat::Cleanliness),
            "Cleaned Polly"
        );
        assert_eq!(
            care_outcome_message(AnimalId::PushPop, AnimalStat::Happiness),
            "Enriched Push Pop"
        );
    }
}
