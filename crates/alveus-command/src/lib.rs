//! Central semantic verb enum and observer-based dispatch.

use bevy::audio::Volume;
use bevy::input_focus::{FocusCause, InputFocus};
use bevy::prelude::*;
use bevy::render::view::screenshot::{Screenshot, save_to_disk};
use bevy::state::state::StateTransition;

use alveus_app::{Menu, Pause, Screen, tile_interaction_enabled_for};
use alveus_components::{MovementController, MovementIntent, Player};
use alveus_interaction::InteractionRequest;
use alveus_menus_models::{ListMenu, ListMenuCursor, ListMenuDirection, ListMenuEntry};
use alveus_screens::PlayRequest;
use alveus_screens::ScreenCommandHandlersPlugin;
use alveus_screens::gameplay::{close_menu_state, open_pause_from_gameplay};
use alveus_stats::{AdvanceTimeRequest, ImproveStatEvent, StatTarget, WorsenStatEvent};
use alveus_types::Stat;
use alveus_world::room::{RoomCommandHandlersPlugin, RoomRequest};

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
/// Triggering is fire-and-forget. Commands route through observers in the same
/// frame they are triggered; [`PendingCommandStateFlush`] queues
/// [`StateTransition`] once per frame phase (First, PostUpdate, and after BRP
/// request processing). Observe the resulting state with the built-in BRP read
/// methods (`world.query`, `world.get_resources`) rather than assuming a
/// synchronous response.
///
/// Variants are grouped into **player verbs** (things a human player can do) and
/// **debug / harness verbs** (mirror in-game debug keys or exist for external
/// control). When editing this enum, keep the per-variant doc comments
/// authoritative: agents read them as the source of truth for what they may do.
#[derive(Event, Debug, Clone, Reflect)]
#[reflect(Event)]
#[type_path = "alveus_headless::command"]
pub enum GameCommand {
    /// Start (or change) the player walking one direction. The player keeps
    /// walking tile-by-tile until [`GameCommand::MoveStop`] is sent or movement
    /// is blocked by an obstacle. One in-flight tile step takes
    /// [`alveus_configs::PLAYER_MOVE_DURATION_SECS`] of sim
    /// time, so in real-time mode hold the intent briefly between stop commands
    /// to advance a single tile predictably. Overlay menus make this a no-op
    /// (list menus use [`GameCommand::NavigateListMenu`] instead). Requires an
    /// active [`Player`] entity.
    Move(MovementIntent),
    /// Clear the player's movement intent (stop walking).
    MoveStop,
    /// Move the cursor on the active list menu one step (`Up` / `Down`).
    /// Applies while [`Menu::Main`], [`Menu::Pause`], or [`Menu::CareItemPicker`]
    /// is open; other menus are a no-op. Care uses the model cursor on
    /// [`CareMenuState`]; Main/Pause update [`ListMenuCursor`] and project
    /// [`InputFocus`] onto the matching row.
    NavigateListMenu(ListMenuDirection),
    /// Interact with whatever is currently in front of / under the player
    /// (`Space` / gamepad South in-game): pick up a `GiveItem`, feed via `FeedAnimal`, enrich via
    /// `EnrichAnimal`, clean via `CleanAnimal`, run a `MiniChore`, open a care
    /// menu (`OpenMenu`), pick up a `PoopPile`, or empty the wheelbarrow at
    /// `PoopDump`. While [`Menu::CareItemPicker`] is open, confirms the
    /// highlighted item instead. Other overlay menus make this a no-op. With no
    /// overlay, it is also a no-op without an active [`Player`] and interaction
    /// target.
    Interact,
    /// Drop the first occupied satchel slot (`K` / gamepad West in-game). No-op if empty.
    /// Requires an active [`Player`] and no overlay menu.
    DropItem,
    /// Enter the building whose entrance the player is standing on (`Enter` /
    /// gamepad North in-game). Only valid while in [`Screen::Gameplay`] and while the player
    /// has a `BuildingEntrance` component (i.e. actually on an entrance tile);
    /// otherwise it is a no-op.
    EnterBuilding,
    /// Leave the current room interior back to the overview (`Backspace` /
    /// gamepad East / walking onto the door in-game). No-op unless currently in an `InRoom`
    /// state. Force-exits regardless of the player's tile within the room.
    ExitRoom,
    /// Toggle the pause menu during gameplay (`P` / `Esc` / gamepad Start).
    PauseToggle,
    /// Press "Play" on the title screen — equivalent to the main-menu button.
    /// Transitions Title → Loading when assets are still pending, otherwise Title → Gameplay.
    Play,
    /// Go back one level in the current menu (`Esc` / `P` / gamepad East or Start): Settings/Credits
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

/// Optional image target used by [`GameCommand::Screenshot`].
///
/// Windowless rendering installs this resource; windowed builds fall back to
/// the primary window when it is absent.
#[derive(Resource, Debug, Clone, Reflect)]
#[reflect(Resource)]
#[type_path = "alveus_headless::camera"]
pub struct HeadlessRenderTarget {
    pub image: Handle<Image>,
    pub width: u32,
    pub height: u32,
}

/// Set when a routed [`GameCommand`] may have queued state transitions.
#[derive(Resource, Default, Debug)]
struct PendingCommandStateFlush(bool);

#[derive(Event, Debug, Clone, Copy)]
enum MovementRequest {
    Move(MovementIntent),
    Stop,
}

#[derive(Event, Debug, Clone, Copy)]
struct ListMenuNavigateRequest(ListMenuDirection);

#[derive(Event, Debug, Clone, Copy)]
enum MenuFlowRequest {
    PauseToggle,
    Back,
    SkipSplash,
    OpenSettings,
    OpenCredits,
    Continue,
    QuitToTitle,
}

#[derive(Event, Debug, Clone)]
struct AdjustVolumeRequest {
    delta: f32,
}

#[derive(Event, Debug, Clone)]
struct ScreenshotRequest {
    path: String,
}

#[derive(Event, Debug, Clone, Copy)]
struct AdvanceFramesRequest(u32);

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
        alveus_app::ensure_plugin(app, ScreenCommandHandlersPlugin);
        alveus_app::ensure_plugin(app, RoomCommandHandlersPlugin);
        app.init_resource::<StepRequest>()
            .init_resource::<PendingCommandStateFlush>()
            .add_observer(route_game_command)
            .add_observer(on_movement_request)
            .add_observer(on_list_menu_navigate_request)
            .add_observer(on_menu_flow_request)
            .add_observer(on_adjust_volume_request)
            .add_observer(on_screenshot_request)
            .add_observer(on_advance_frames_request)
            .add_systems(First, flush_command_state_transitions)
            .add_systems(PostUpdate, flush_command_state_transitions);

        #[cfg(feature = "remote")]
        {
            use bevy::remote::{RemoteLast, RemoteSystems};
            app.add_systems(
                RemoteLast,
                flush_command_state_transitions.after(RemoteSystems::ProcessRequests),
            );
        }
    }
}

fn route_game_command(
    trigger: On<GameCommand>,
    screen: Res<State<Screen>>,
    menu: Res<State<Menu>>,
    mut commands: Commands,
    mut flush: ResMut<PendingCommandStateFlush>,
) {
    let command = trigger.event().clone();

    if *screen.get() == Screen::FatalError {
        match &command {
            GameCommand::Screenshot { .. } | GameCommand::AdvanceFrames(_) => {}
            _ => return,
        }
    }

    match command {
        GameCommand::Move(intent) => {
            commands.trigger(MovementRequest::Move(intent));
        }
        GameCommand::MoveStop => {
            commands.trigger(MovementRequest::Stop);
        }
        GameCommand::NavigateListMenu(direction) => {
            if *menu.get() == Menu::CareItemPicker {
                commands.trigger(InteractionRequest::NavigateCareMenu(direction));
            } else {
                commands.trigger(ListMenuNavigateRequest(direction));
            }
        }
        GameCommand::Interact => {
            commands.trigger(InteractionRequest::Interact);
        }
        GameCommand::DropItem => {
            commands.trigger(InteractionRequest::DropItem);
        }
        GameCommand::EnterBuilding => {
            commands.trigger(RoomRequest::EnterBuilding);
        }
        GameCommand::ExitRoom => {
            commands.trigger(RoomRequest::ExitRoom);
        }
        GameCommand::PauseToggle => {
            commands.trigger(MenuFlowRequest::PauseToggle);
        }
        GameCommand::Play => {
            commands.trigger(PlayRequest);
        }
        GameCommand::Back => {
            if *menu.get() == Menu::CareItemPicker {
                commands.trigger(InteractionRequest::CancelCareMenu);
            } else {
                commands.trigger(MenuFlowRequest::Back);
            }
        }
        GameCommand::SkipSplash => {
            commands.trigger(MenuFlowRequest::SkipSplash);
        }
        GameCommand::OpenSettings => {
            commands.trigger(MenuFlowRequest::OpenSettings);
        }
        GameCommand::OpenCredits => {
            commands.trigger(MenuFlowRequest::OpenCredits);
        }
        GameCommand::Continue => {
            if *menu.get() == Menu::CareItemPicker {
                commands.trigger(InteractionRequest::ConfirmCareMenu);
            } else {
                commands.trigger(MenuFlowRequest::Continue);
            }
        }
        GameCommand::QuitToTitle => {
            commands.trigger(MenuFlowRequest::QuitToTitle);
        }
        GameCommand::ImproveStat { target, amount } => {
            commands.trigger(ImproveStatEvent { target, amount });
        }
        GameCommand::WorsenStat { target, amount } => {
            commands.trigger(WorsenStatEvent { target, amount });
        }
        GameCommand::AdvanceTime { hours } => {
            commands.trigger(AdvanceTimeRequest { hours });
        }
        GameCommand::AdjustVolume { delta } => {
            commands.trigger(AdjustVolumeRequest { delta });
        }
        GameCommand::Screenshot { path } => {
            commands.trigger(ScreenshotRequest { path });
        }
        GameCommand::AdvanceFrames(frames) => {
            commands.trigger(AdvanceFramesRequest(frames));
        }
    }

    flush.0 = true;
}

fn flush_command_state_transitions(
    mut commands: Commands,
    mut flush: ResMut<PendingCommandStateFlush>,
) {
    if flush.0 {
        flush.0 = false;
        commands.run_schedule(StateTransition);
    }
}

fn on_movement_request(
    trigger: On<MovementRequest>,
    screen: Res<State<Screen>>,
    menu: Res<State<Menu>>,
    mut players: Query<&mut MovementController, With<Player>>,
) {
    match trigger.event() {
        MovementRequest::Move(intent) => {
            if !tile_interaction_enabled_for(*screen.get(), *menu.get()) {
                return;
            }
            if let Ok(mut movement) = players.single_mut() {
                movement.intent = Some(*intent);
            }
        }
        MovementRequest::Stop => {
            if let Ok(mut movement) = players.single_mut() {
                movement.intent = None;
            }
        }
    }
}

fn on_list_menu_navigate_request(
    trigger: On<ListMenuNavigateRequest>,
    menu: Res<State<Menu>>,
    mut list_cursors: Query<(Entity, &mut ListMenuCursor), With<ListMenu>>,
    entries: Query<(Entity, &ListMenuEntry)>,
    mut focus: Option<ResMut<InputFocus>>,
) {
    let direction = trigger.event().0;
    let delta = direction.delta();
    match *menu.get() {
        Menu::Main | Menu::Pause => {
            let Ok((list, mut cursor)) = list_cursors.single_mut() else {
                return;
            };
            cursor.move_by(delta);
            let index = cursor.index;
            if let Some(entity) = entries.iter().find_map(|(entity, entry)| {
                (entry.list == list && entry.index == index).then_some(entity)
            }) && let Some(focus) = focus.as_mut()
            {
                focus.set(entity, FocusCause::Navigated);
            }
        }
        Menu::None | Menu::Settings | Menu::Credits | Menu::CareItemPicker => {}
    }
}

fn on_menu_flow_request(
    trigger: On<MenuFlowRequest>,
    screen: Res<State<Screen>>,
    menu: Res<State<Menu>>,
    mut next_screen: ResMut<NextState<Screen>>,
    mut next_menu: ResMut<NextState<Menu>>,
    mut next_pause: ResMut<NextState<Pause>>,
) {
    match trigger.event() {
        MenuFlowRequest::PauseToggle => match (*screen.get(), *menu.get()) {
            (Screen::Gameplay, Menu::None) => {
                open_pause_from_gameplay(&mut next_pause, &mut next_menu);
            }
            (Screen::Gameplay, _) => {
                close_menu_state(&mut next_menu);
            }
            _ => {}
        },
        MenuFlowRequest::Back => {
            go_back_menu(screen.get(), menu.get(), &mut next_menu);
        }
        MenuFlowRequest::SkipSplash => {
            next_screen.set(Screen::Title);
        }
        MenuFlowRequest::OpenSettings => {
            next_menu.set(Menu::Settings);
        }
        MenuFlowRequest::OpenCredits => {
            next_menu.set(Menu::Credits);
        }
        MenuFlowRequest::Continue => {
            close_menu_state(&mut next_menu);
        }
        MenuFlowRequest::QuitToTitle => {
            next_screen.set(Screen::Title);
        }
    }
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

fn on_adjust_volume_request(
    trigger: On<AdjustVolumeRequest>,
    mut global_volume: Option<ResMut<GlobalVolume>>,
) {
    const MIN_VOLUME: f32 = 0.0;
    const MAX_VOLUME: f32 = 3.0;
    let Some(global_volume) = global_volume.as_mut() else {
        return;
    };
    let linear =
        (global_volume.volume.to_linear() + trigger.event().delta).clamp(MIN_VOLUME, MAX_VOLUME);
    global_volume.volume = Volume::Linear(linear);
}

fn on_screenshot_request(
    trigger: On<ScreenshotRequest>,
    render_target: Option<Res<HeadlessRenderTarget>>,
    mut commands: Commands,
) {
    let path = trigger.event().path.clone();
    if let Some(target) = render_target {
        commands
            .spawn(Screenshot::image(target.image.clone()))
            .observe(save_to_disk(path));
    } else {
        commands
            .spawn(Screenshot::primary_window())
            .observe(save_to_disk(path));
    }
}

fn on_advance_frames_request(
    trigger: On<AdvanceFramesRequest>,
    mut step_request: ResMut<StepRequest>,
) {
    step_request.add(trigger.event().0);
}

#[cfg(test)]
mod tests {
    use super::GameCommand;
    use bevy::reflect::TypePath;

    #[test]
    fn game_command_keeps_legacy_brp_type_path() {
        assert_eq!(
            GameCommand::type_path(),
            "alveus_headless::command::GameCommand"
        );
    }
}
