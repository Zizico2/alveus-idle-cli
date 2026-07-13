# Alveus Idle CLI

A 2D tile-based simulation/idle game built in Rust using the **Bevy** game engine. The game is inspired by and themed around **Alveus Sanctuary**, the nonprofit wildlife sanctuary and virtual education center founded by streamer Maya Higa.

### Design & roadmap

| Need | Start here |
|------|------------|
| What to build next / lore | [`ROADMAP.md`](ROADMAP.md) |
| Shipped gameplay numbers | [`crates/alveus-configs`](crates/alveus-configs) |
| Not-yet-shipped ballparks | [`crates/alveus-configs/README.md`](crates/alveus-configs/README.md) |
| Historical intent (markdown only) | [`design/`](design/) |

`design/` is inspiration, not a contract. **Siren** is a Blue-fronted Amazon memorial ambassador (see ROADMAP lore table) — not a ball python.

---

## 🎮 Game Concept & Overview

**Alveus Idle CLI** invites players to explore a digital recreation of Alveus Sanctuary. Players control an animal ambassador caretaker (conceptually represented by a duck, using `ducky.png` in the assets) and navigate the sanctuary grounds, entering various operational buildings to perform caretaking tasks.

### Core Mechanics
* **Grid-Based Tile Movement:** The player navigates a 32x32 pixel grid. Movement is snapped to individual tiles with smooth transition logic (`crates/alveus-world/src/movement.rs`).
* **Obstacle Collision:** Movement is restricted by `Obstacle` components, which dynamically block navigation through walls, boundaries, and heavy furniture.
* **Building Entrances & Transitions:** Stepping onto designated building entrances triggers a responsive slide-in toast notification prompting the player to enter. Pressing `Enter` transitions the player state into the room interior, while pressing `Backspace` or walking to the exit door returns them to the outdoor overview.
* **Ambient Audio:** The game features dynamic footstep sound effects (`step1.ogg` through `step4.ogg`) that play in rhythm with player steps.

---

## 🗺️ The Map & Assets

The game's maps are built using the **Tiled Map Editor** and imported directly into Bevy using `bevy_ecs_tiled`.

### Overview Map (`assets/maps/overview/`)
* **Terrain:** Composed of three primary tile types:
  * `sand_tile.png` (Sandy pathways)
  * `sand_grass_tile.png` (Path-to-grass transitions)
  * `grass_tile.png` (Lush sanctuary grass)
* **Nutrition House:** The main building visible on the overview map, rendered using a custom `nutrition_house.png` (200x284 pixels) asset.

### Building Interior: The Nutrition House
The **Nutrition House** is the sanctuary's food preparation hub. Inside this room, players find key items related to animal diet prep:
1. **Smoothie Counter:** Where fruits and vegetables are prepared and blended into nutritious smoothies for the animal ambassadors.
2. **Herb Garden Patch:** A patch used to grow fresh greens and herbs.
3. **Seed Chest:** A storage chest containing seeds and grains for sanctuary birds.

---

## 🛠️ Tech Stack & Dependencies

* **[Bevy (v0.18)](https://bevyengine.org/):** A refreshingly simple data-driven game engine in Rust.
* **[bevy_ecs_tiled (v0.11.2)](https://github.com/vleue/bevy_ecs_tiled):** ECS-friendly integration for Tiled map files, exposing Tiled object properties directly as Bevy components.
* **[bevy_tweening (v0.15)](https://github.com/HeavyDutyApps/bevy_tweening):** Used for smooth UI animations, such as the entering/exiting toast notifications.

---

## 📂 Project Structure

Cargo workspace: thin binary in `src/`, feature crates under `crates/`.

* `src/main.rs` / `src/lib.rs`: Composition root — `build_app`, run modes, plugin wiring.
* `src/bin/gen_tiled_types.rs`: Regenerates `assets/maps/overview/tiled_types.json`.
* `crates/alveus-app`: App-wide states (`Screen`, `Menu`, `Pause`, `InRoom`) and `AppSystems`.
* `crates/alveus-components` / `alveus-content` / `alveus-types` / `alveus-configs`: Shared data and content.
* `crates/alveus-world`: Level, player, movement, rooms, entrances, camera, toast.
* `crates/alveus-stats` / `alveus-cleaning` / `alveus-interaction` / `alveus-animals` / `alveus-collision`: Gameplay plugins.
* `crates/alveus-screens` / `alveus-menus` / `alveus-hud` / `alveus-theme`: UI.
* `crates/alveus-command` / `alveus-input` / `alveus-reflect`: semantic verbs,
  keyboard mapping, and agent-facing type registration.
* `crates/alveus-headless`: offscreen camera plus HTTP/stdio BRP transports.
* `assets/`:
  * `images/`: Sprite and UI textures (such as `ducky.png`).
  * `maps/overview/`: Tiled `.tmx` maps, tilesets, and exports.
  * `audio/`: Footsteps and sound effects.

---

## 🚀 Getting Started

### Prerequisites
Make sure you have Rust and Cargo installed. If not, get them at [rustup.rs](https://rustup.rs/).
For the recipes below, also install [`just`](https://github.com/casey/just).

### Running the Game (Native)
Preferred local loop — regenerates `tiled_types.json`, then launches the game (hot-reloading and Bevy dev tools via default features):
```bash
just dev-run
```

If Tiled Reflect types haven’t changed, you can skip the export and run directly:
```bash
cargo run
# or: just run
```

### Building for Release
To compile a highly-optimized release build:
```bash
cargo build --release
```

---

## 🤖 Headless / Remote Control (BRP)

The game can run **windowless** and be driven entirely by external clients (LLMs, scripts, CI) over **Bevy Remote Protocol (BRP)** — JSON-RPC 2.0 — with **no custom methods** and **no bespoke observation snapshot**. Commands are semantic verbs on the reflected `GameCommand` event; observation uses built-in `world.query`, `world.get_resources`, and `registry.schema`.

### Build & run

```bash
# Compile with headless support (remote HTTP + stdio BRP)
cargo run --features headless -- --headless

# Deterministic frame stepping (blocks until client sends AdvanceFrames)
cargo run --features headless -- --headless --step

# Options
cargo run --features headless -- --headless --port 15702 --resolution 1280x720 --no-stdio
```

| Flag | Description |
|------|-------------|
| `--headless` | Windowless mode: offscreen render target, BRP HTTP, stdio pipe |
| `--step` | Manual step loop (`GameCommand::AdvanceFrames`) |
| `--realtime` | Real-time metronome (default when `--step` omitted) |
| `--port N` | BRP HTTP port (default `15702`) |
| `--resolution WxH` | Offscreen camera / screenshot size (default `1280x720`) |
| `--no-stdio` | Disable stdin/stdout JSON-RPC carrier |

### Transports (one protocol, two carriers)

1. **HTTP** — `RemoteHttpPlugin` on `--port` (default `15702`).
2. **Stdio** — one JSON-RPC object per line on stdin; responses on stdout (same methods as HTTP).

### Commands (`world.trigger_event`)

Trigger the registered event `alveus_headless::command::GameCommand`:

```json
{
  "jsonrpc": "2.0",
  "method": "world.trigger_event",
  "id": 1,
  "params": {
    "event": "alveus_headless::command::GameCommand",
    "value": "SkipSplash"
  }
}
```

**Verb variants** (semantic only — no key injection):

| Category | Variants |
|----------|----------|
| Locomotion | `Move` / `MoveStop` |
| Interaction | `Interact`, `DropItem` |
| Buildings | `EnterBuilding`, `ExitRoom` |
| Flow | `PauseToggle`, `Play`, `Back`, `SkipSplash`, `OpenSettings`, `OpenCredits`, `Continue`, `QuitToTitle` |
| Stats / time | `ImproveStat { target, amount }`, `WorsenStat { … }`, `AdvanceTime { hours }` |
| Settings | `AdjustVolume { delta }` |
| Capture | `Screenshot { path }` |
| Frame control | `AdvanceFrames(n)` (step mode) |

Use `registry.schema` / `rpc.discover` to introspect exact Reflect shapes for struct variants.

### Observation (built-in BRP only)

- `world.get_resources` — e.g. `State<Screen>`, `State<Menu>`, `Pause`, `PlayerSatchel`, `ActiveInteractionTarget`, `SanctuaryUpkeep`
- `world.query` — player position, animals, enclosures, interactables
- `registry.schema` — full type system for clients

Derived facts (adjacency, cleanliness joins, etc.) are computed **client-side** from query results.

### Screenshots

Offscreen mode renders to an `Image` render target (no display server, no Xvfb).
The headless camera is the default UI camera, so captures are the **composed
frame** (world geometry plus HUD, menus, and toasts) at the configured
`--resolution`. Capture via:

```json
{
  "method": "world.trigger_event",
  "params": {
    "event": "alveus_headless::command::GameCommand",
    "value": { "Screenshot": { "path": "/abs/path/to/repo/screenshots/frame.png" } }
  }
}
```

Prefer writing under `screenshots/` (see `scripts/headless_ui_screenshot_smoke.py`,
which asserts UI overlay pixels and fails on world-only captures). Use ECS
queries for logic assertions; treat PNGs as presentation checks. Visual pixel
tests need a **wgpu device** (GPU or lavapipe) and stay outside plain CI.

### Tests

```bash
cargo test --profile ci
cargo test --features headless --profile ci
```

Headless integration tests cover `GameCommand` dispatch, BRP in-process round-trips, stdio-equivalent JSON-RPC, and reflect registry presence. Render/screenshot tests require a **wgpu device** (GPU runner or software Vulkan/lavapipe); the app itself is windowless.

### Module layout

* `crates/alveus-command/src/lib.rs` — `GameCommand` enum + dispatcher
* `crates/alveus-input/src/lib.rs` — keyboard-to-command mapping
* `crates/alveus-reflect/src/lib.rs` — `register_agent_types()` for BRP introspection
* `crates/alveus-headless/src/camera.rs` — offscreen `Camera2d` → `RenderTarget::Image`
* `crates/alveus-headless/src/stdio.rs` — stdin/stdout BRP carrier
