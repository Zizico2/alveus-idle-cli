//! Central semantic verb enum and dispatcher.

use bevy::audio::Volume;
use bevy::prelude::*;
use bevy::render::view::screenshot::{Screenshot, save_to_disk};
use bevy::state::state::StateTransition;

use alveus_app::{InRoom, Menu, Pause, Screen, tile_interaction_enabled_for};
use alveus_components::{
    BuildingEntrance, CurrentTilePosition, MovementController, MovementIntent, Player, TilePosition,
};
use alveus_interaction::{
    CareMenuState, cancel_care_menu_in_world, care_menu_move_cursor, confirm_care_menu_in_world,
    perform_drop_in_world, perform_interact_in_world,
};
use alveus_menus::PlayClickEvent;
use alveus_screens::gameplay::spawn_pause_overlay;
use alveus_stats::{ImproveStatEvent, StatTarget, WorsenStatEvent, advance_simulated_hours_world};
use alveus_types::Stat;
use alveus_world::room::{PlayerSpawnPoint, try_enter_room};

use crate::camera::HeadlessRenderTarget;

/// The complete, semantic verb set for the game.
///
/// This enum **is** the public API for any external controller (LLM, script,
/// test). Every action a player can take has exactly one variant here, and the
/// same variant is produced by both the keyboard readers and by external clients
/// via BRP's `world.trigger_event` (event path
/// `alveus_headless::command::GameCommand`). There is deliberately **no**
/// key-injection escape hatch and **no** higher-level convenience verb (e.g. no
/// `MoveTo(tile)`): an agent must accomplish goals using the same primitives a
/// human player has, one tile step at a time.
///
/// Triggering is fire-and-forget. Commands are buffered and applied once per
/// frame (see [`apply_pending_game_commands`]); observe the resulting state with
/// the built-in BRP read methods (`world.query`, `world.get_resources`) rather
/// than assuming a synchronous response.
///
/// Variants are grouped into **player verbs** (things a human player can do) and
/// **debug / harness verbs** (mirror in-game debug keys or exist for external
/// control). When editing this enum, keep the per-variant doc comments
/// authoritative: agents read them as the source of truth for what they may do.
#[derive(Event, Debug, Clone, Reflect)]
#[reflect(Event)]
pub enum GameCommand {
    /// Start (or change) the player walking one direction. The player keeps
    /// walking tile-by-tile until [`GameCommand::MoveStop`] is sent or movement
    /// is blocked by an obstacle. One in-flight tile step takes
    /// [`alveus_configs::PLAYER_MOVE_DURATION_SECS`] of sim
    /// time, so in real-time mode hold the intent briefly between stop commands
    /// to advance a single tile predictably. While [`Menu::CareItemPicker`] is
    /// open, Up/Down moves its cursor instead; other overlay menus make this a
    /// no-op. Requires an active [`Player`] entity.
    Move(MovementIntent),
    /// Clear the player's movement intent (stop walking).
    MoveStop,
    /// Interact with whatever is currently in front of / under the player
    /// (`Space` in-game): pick up a `GiveItem`, feed via `FeedAnimal`, enrich via
    /// `EnrichAnimal`, clean via `CleanAnimal`, run a `MiniChore`, open a care
    /// menu (`OpenMenu`), pick up a `PoopPile`, or empty the wheelbarrow at
    /// `PoopDump`. While [`Menu::CareItemPicker`] is open, confirms the
    /// highlighted item instead. Other overlay menus make this a no-op. With no
    /// overlay, it is also a no-op without an active [`Player`] and interaction
    /// target.
    Interact,
    /// Drop the first occupied satchel slot (`K` in-game). No-op if empty.
    /// Requires an active [`Player`] and no overlay menu.
    DropItem,
    /// Enter the building whose entrance the player is standing on (`Enter`
    /// in-game). Only valid while in [`Screen::Gameplay`] and while the player
    /// has a `BuildingEntrance` component (i.e. actually on an entrance tile);
    /// otherwise it is a no-op.
    EnterBuilding,
    /// Leave the current room interior back to the overview (`Backspace` /
    /// walking onto the door in-game). No-op unless currently in an `InRoom`
    /// state. Force-exits regardless of the player's tile within the room.
    ExitRoom,
    /// Toggle the pause menu during gameplay (`P` / `Esc`).
    PauseToggle,
    /// Press "Play" on the title screen — equivalent to the main-menu button.
    /// Transitions Title -> Gameplay.
    Play,
    /// Go back one level in the current menu (`Esc` in menus): Settings/Credits
    /// -> previous menu, Pause -> resume, CareItemPicker -> close picker.
    Back,
    /// Skip the splash screen (`Esc` during splash). Transitions to Title.
    SkipSplash,
    /// Open the Settings menu.
    OpenSettings,
    /// Open the Credits menu.
    OpenCredits,
    /// Close the active menu and continue playing (resume from pause).
    /// While [`Menu::CareItemPicker`] is open, confirms the highlighted item
    /// (same as Interact in that menu).
    Continue,
    /// Abandon the current session and return to the Title screen.
    QuitToTitle,

    // --- Debug / harness verbs below ---
    // These mirror the in-game debug keybindings or exist purely for external
    // control. They are NOT things an ordinary player does during normal play;
    // prefer the player verbs above when reproducing real play sessions.
    /// Debug: increase a specific animal/enclosure stat (mirrors debug keys).
    /// Triggers an [`ImproveStatEvent`]. `amount` is a direction-agnostic
    /// [`Stat`] quantity on the shared scale; on the BRP wire it serializes as a
    /// bare `u32` (not a `{ "0": … }` map).
    ImproveStat { target: StatTarget, amount: Stat },
    /// Debug: decrease a specific animal/enclosure stat (mirrors debug keys).
    /// Triggers a [`WorsenStatEvent`]. `amount` is a direction-agnostic
    /// [`Stat`] quantity on the shared scale; on the BRP wire it serializes as a
    /// bare `u32` (not a `{ "0": … }` map).
    WorsenStat { target: StatTarget, amount: Stat },
    /// Debug: fast-forward simulated decay by `hours` (mirrors the `=`/`L`
    /// fast-forward path).
    AdvanceTime { hours: f32 },
    /// Debug: nudge the global audio volume by `delta` (clamped to [0.0, 3.0]).
    AdjustVolume { delta: f32 },
    /// Harness: capture a PNG of the composed offscreen frame (world + UI) to
    /// `path`. The headless camera is the default UI camera, so HUD, menus, and
    /// toasts are included. With no headless render target it falls back to the
    /// primary window. The write is asynchronous — wait a couple of frames
    /// before reading the file. Visual PNG checks need a wgpu adapter; use ECS
    /// queries as the source of truth for game logic.
    Screenshot { path: String },
    /// Harness: in `--step` mode, request that the headless loop simulate `n`
    /// additional frames. Ignored in `--realtime` mode (frames advance on a
    /// wall-clock metronome there).
    AdvanceFrames(u32),
}

/// Frames the headless loop should simulate before blocking again (step mode).
#[derive(Resource, Default, Debug, Reflect)]
#[reflect(Resource)]
pub struct StepRequest {
    pub pending: u32,
}

/// Commands collected by the observer and applied before state transitions.
#[derive(Resource, Default, Debug)]
pub struct PendingGameCommands(pub Vec<GameCommand>);

impl StepRequest {
    pub fn add(&mut self, frames: u32) {
        self.pending = self.pending.saturating_add(frames);
    }

    pub fn take_one(&mut self) -> bool {
        if self.pending > 0 {
            self.pending -= 1;
            true
        } else {
            false
        }
    }

    pub fn take_all(&mut self) -> u32 {
        let taken = self.pending;
        self.pending = 0;
        taken
    }
}

/// Routes semantic [`GameCommand`] events into gameplay actions.
///
/// Requires [`alveus_app::plugin`] to initialize the app-wide states first.
pub struct CommandPlugin;

impl Plugin for CommandPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<StepRequest>()
            .init_resource::<PendingGameCommands>()
            .add_observer(enqueue_game_command)
            .add_systems(First, apply_pending_game_commands)
            .add_systems(PostUpdate, apply_pending_game_commands);

        #[cfg(feature = "remote")]
        {
            use bevy::remote::{RemoteLast, RemoteSystems};
            app.add_systems(
                RemoteLast,
                apply_pending_game_commands.after(RemoteSystems::ProcessRequests),
            );
        }
    }
}

fn enqueue_game_command(trigger: On<GameCommand>, mut pending: ResMut<PendingGameCommands>) {
    pending.0.push(trigger.event().clone());
}

fn apply_pending_game_commands(world: &mut World) {
    let commands = std::mem::take(&mut world.resource_mut::<PendingGameCommands>().0);
    if commands.is_empty() {
        return;
    }
    for command in commands {
        apply_game_command(world, command);
    }
    let _ = world.try_run_schedule(StateTransition);
}

fn care_menu_open(world: &World) -> bool {
    world
        .get_resource::<State<Menu>>()
        .is_some_and(|menu| *menu.get() == Menu::CareItemPicker)
}

fn has_player(world: &World) -> bool {
    world
        .iter_entities()
        .filter(|entity| entity.contains::<Player>())
        .count()
        == 1
}

fn apply_game_command(world: &mut World, command: GameCommand) {
    // FatalError is terminal for this process: ignore gameplay / navigation verbs.
    // Passive harness verbs (screenshots, frame advance) still apply.
    if *world.resource::<State<Screen>>().get() == Screen::FatalError {
        match &command {
            GameCommand::Screenshot { .. } | GameCommand::AdvanceFrames(_) => {}
            _ => return,
        }
    }

    match command {
        GameCommand::Move(intent) => {
            if care_menu_open(world) {
                if !has_player(world) {
                    return;
                }
                {
                    let mut query = world.query_filtered::<&mut MovementController, With<Player>>();
                    if let Ok(mut movement) = query.single_mut(world) {
                        movement.intent = None;
                    }
                }
                let delta = match intent {
                    MovementIntent::Up => -1,
                    MovementIntent::Down => 1,
                    MovementIntent::Left | MovementIntent::Right => 0,
                };
                if delta != 0 {
                    let mut care_menu = world.resource_mut::<CareMenuState>();
                    care_menu_move_cursor(&mut care_menu, delta);
                }
                return;
            }
            let screen = *world.resource::<State<Screen>>().get();
            let menu = *world.resource::<State<Menu>>().get();
            if !tile_interaction_enabled_for(screen, menu) {
                return;
            }
            let mut query = world.query_filtered::<&mut MovementController, With<Player>>();
            if let Ok(mut movement) = query.single_mut(world) {
                movement.intent = Some(intent);
            }
        }
        GameCommand::MoveStop => {
            let mut query = world.query_filtered::<&mut MovementController, With<Player>>();
            if let Ok(mut movement) = query.single_mut(world) {
                movement.intent = None;
            }
        }
        GameCommand::Interact => perform_interact_in_world(world),
        GameCommand::DropItem => perform_drop_in_world(world),
        GameCommand::EnterBuilding => {
            let screen = *world.resource::<State<Screen>>().get();
            let menu = *world.resource::<State<Menu>>().get();
            if screen != Screen::Gameplay || !tile_interaction_enabled_for(screen, menu) {
                return;
            }
            let entrance = {
                let mut entrance_query = world.query_filtered::<&BuildingEntrance, With<Player>>();
                entrance_query.single(world).ok().copied()
            };
            let Some(entrance) = entrance else {
                return;
            };
            let mut next_screen = world.resource_mut::<NextState<Screen>>();
            match entrance {
                BuildingEntrance::NutritionHouse => {
                    try_enter_room(
                        &entrance,
                        BuildingEntrance::NutritionHouse,
                        Screen::InRoom(InRoom::NutritionHouse),
                        &mut next_screen,
                    );
                }
                BuildingEntrance::PushPopEnclosure => {
                    try_enter_room(
                        &entrance,
                        BuildingEntrance::PushPopEnclosure,
                        Screen::InRoom(InRoom::PushPopEnclosure),
                        &mut next_screen,
                    );
                }
                BuildingEntrance::NoEntrance => {}
            }
        }
        GameCommand::ExitRoom => {
            let screen = *world.resource::<State<Screen>>().get();
            let menu = *world.resource::<State<Menu>>().get();
            if !tile_interaction_enabled_for(screen, menu) {
                return;
            }
            let player_pos = {
                let mut pos_query = world.query_filtered::<&CurrentTilePosition, With<Player>>();
                pos_query.single(world).ok().map(|pos| pos.0)
            };
            let Some(player_pos) = player_pos else {
                return;
            };
            match screen {
                Screen::InRoom(InRoom::NutritionHouse) => {
                    exit_room_world(world, player_pos, TilePosition { x: 33, y: 12 });
                }
                Screen::InRoom(InRoom::PushPopEnclosure) => {
                    exit_room_world(world, player_pos, TilePosition { x: 40, y: 33 });
                }
                _ => {}
            }
        }
        GameCommand::PauseToggle => {
            let screen = *world.resource::<State<Screen>>().get();
            let menu = *world.resource::<State<Menu>>().get();
            match (screen, menu) {
                (Screen::Gameplay, Menu::None) => {
                    world.resource_mut::<NextState<Pause>>().set(Pause(true));
                    spawn_pause_overlay(&mut world.commands());
                    world.resource_mut::<NextState<Menu>>().set(Menu::Pause);
                }
                (Screen::Gameplay, _) => {
                    world.resource_mut::<NextState<Menu>>().set(Menu::None);
                }
                _ => {}
            }
        }
        GameCommand::Play => {
            world.trigger(PlayClickEvent);
        }
        GameCommand::Back => {
            let screen = *world.resource::<State<Screen>>().get();
            let menu = *world.resource::<State<Menu>>().get();
            if menu == Menu::CareItemPicker {
                cancel_care_menu_in_world(world);
                return;
            }
            let mut next_menu = world.resource_mut::<NextState<Menu>>();
            go_back_menu(&screen, &menu, &mut next_menu);
        }
        GameCommand::SkipSplash => {
            world.resource_mut::<NextState<Screen>>().set(Screen::Title);
        }
        GameCommand::OpenSettings => {
            world.resource_mut::<NextState<Menu>>().set(Menu::Settings);
        }
        GameCommand::OpenCredits => {
            world.resource_mut::<NextState<Menu>>().set(Menu::Credits);
        }
        GameCommand::Continue => {
            if care_menu_open(world) {
                confirm_care_menu_in_world(world);
                return;
            }
            world.resource_mut::<NextState<Menu>>().set(Menu::None);
        }
        GameCommand::QuitToTitle => {
            world.resource_mut::<NextState<Screen>>().set(Screen::Title);
        }
        GameCommand::ImproveStat { target, amount } => {
            world.trigger(ImproveStatEvent { target, amount });
        }
        GameCommand::WorsenStat { target, amount } => {
            world.trigger(WorsenStatEvent { target, amount });
        }
        GameCommand::AdvanceTime { hours } => {
            advance_simulated_hours_world(world, hours);
        }
        GameCommand::AdjustVolume { delta } => {
            const MIN_VOLUME: f32 = 0.0;
            const MAX_VOLUME: f32 = 3.0;
            let mut global_volume = world.resource_mut::<GlobalVolume>();
            let linear = (global_volume.volume.to_linear() + delta).clamp(MIN_VOLUME, MAX_VOLUME);
            global_volume.volume = Volume::Linear(linear);
        }
        GameCommand::Screenshot { path } => {
            let screenshot_target = world
                .get_resource::<HeadlessRenderTarget>()
                .map(|target| target.image.clone());
            let mut commands = world.commands();
            if let Some(image) = screenshot_target {
                commands
                    .spawn(Screenshot::image(image))
                    .observe(save_to_disk(path));
            } else {
                commands
                    .spawn(Screenshot::primary_window())
                    .observe(save_to_disk(path));
            }
        }
        GameCommand::AdvanceFrames(frames) => {
            world.resource_mut::<StepRequest>().add(frames);
        }
    }
}

fn exit_room_world(world: &mut World, _player_pos: TilePosition, exit_spawn: TilePosition) {
    info!("Exiting room interior!");
    world.resource_mut::<PlayerSpawnPoint>().position = exit_spawn;
    world
        .resource_mut::<NextState<Screen>>()
        .set(Screen::Gameplay);
}

fn go_back_menu(screen: &Screen, menu: &Menu, next_menu: &mut NextState<Menu>) {
    match menu {
        Menu::Settings => {
            next_menu.set(if screen == &Screen::Title {
                Menu::Main
            } else {
                Menu::Pause
            });
        }
        Menu::Credits => next_menu.set(Menu::Main),
        Menu::Pause | Menu::CareItemPicker => next_menu.set(Menu::None),
        Menu::Main | Menu::None => {}
    }
}
