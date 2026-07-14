# AGENTS.md — How agents should work in this repo

This document is the operating manual for AI agents (and humans pairing with them)
working on **alveus-idle-cli**. It complements the README's "Headless / Remote
Control (BRP)" section. Read both before starting.

The core idea: this game is **agent-native**. It can run windowless and be driven
entirely over Bevy Remote Protocol (BRP, JSON-RPC 2.0). An agent plays and tests
the game the same way a player does — through semantic verbs — and observes state
primarily through generic ECS queries, with screenshots as a visual supplement.
There is no bespoke "agent API" to keep in sync; the ECS *is* the API.

---

## 0. Design docs (three-layer SoT)

| Layer | Owns |
|-------|------|
| [`ROADMAP.md`](ROADMAP.md) | Epic backlog, handoff briefs, lore corrections (Siren parrot memorial) |
| [`crates/alveus-configs`](crates/alveus-configs) | **All gameplay numbers** — Rust for shipped; crate `README.md` Planned section for ballparks |
| [`design/`](design/) | Markdown-only historical intent / copy — **not** binding; no JSON |

**Workflow:** pick an epic from the roadmap → write an implementation plan → put new numbers in `alveus-configs` (promote from Planned into Rust) → code + tests. Do **not** invent magic locals in feature crates, and do not regenerate design JSON (there is none). Re-read code and configs; do not trust stale coords in old markdown sketches.

**Siren:** Blue-fronted Amazon, memorial/legacy — not a ball python. Snake shed → Epic 12 (Noodle/Patchy).

---

## 1. Golden rules

1. **Play like a player.** Drive the game only through the [`GameCommand`] verb
   set. Do not add key-injection hatches, and do not add higher-level convenience
   verbs such as `MoveTo(tile)` or pathfinding. If a human can't do it with one
   keypress, an agent shouldn't get a shortcut for it either. Walk tile-by-tile
   with `Move`/`MoveStop`, just like a player.
2. **The verb list is the enum.** The curated, canonical list of things an agent
   may do is `GameCommand` in `crates/alveus-command/src/lib.rs`. There is no separate
   catalogue. When you add or change a verb, update its doc comment — those
   comments are the source of truth agents read.
3. **No custom BRP methods, no bespoke observation struct.** Commands go through
   the built-in `world.trigger_event`; structured observation uses the built-in
   `world.query` / `world.get_resources` / `registry.schema` / `rpc.discover`.
   Visual observation uses `GameCommand::Screenshot` (see §3). Anything an agent
   must query or trigger must derive `Reflect` and be registered (see §7).
4. **Drive the game from a Python script, not ad-hoc shell calls.** See §4. This
   is the most important workflow rule.
5. **Lock behavior in with tests.** Once a flow works in a throwaway script,
   promote the important assertions into Rust tests (unit + BRP e2e). See §5.
6. **Treat this document's literals as hints, not contracts.** Spawn tiles,
   entrance bounds, ports, file paths, type paths, and line-number citations
   are **examples as of when this file was written**. They can change at any time.
   Before relying on a value, **read it from the codebase** (or derive it at
   runtime via BRP/logs). See §4 for where to look.
7. **Verify position after every move.** After each `Move`/`MoveStop` cycle,
   query `CurrentTilePosition` via `world.query` and use the returned tile as
   ground truth. A `Move` command does **not** guarantee the player advanced —
   static obstacles, room walls, and **dynamic obstacles** (wandering animals,
   spawned poop piles, etc.) can block the intended tile. Tile-counting is
   useful for planning routes; it is never a substitute for reading position
   back from the ECS.
8. **Delegate mundane verbose work** when you are a high-cost lead agent.
   See [Cost-tier delegation](#cost-tier-delegation) below.

---

## Cost-tier delegation

High-cost lead agents should **not** burn context on long, mechanical, or
high-volume chores. Spawn a cheaper worker subagent for those, keep the
reasoning/design loop on the lead, and **require a fixed summary shape** so the
lead never has to re-ingest raw logs.

### Who this applies to (leads)

| Family | Lead (expensive) — do design / judgment here |
|--------|-----------------------------------------------|
| Claude | Fable, Opus |
| Codex / GPT | 5.6 Sol, 5.6 Terra |
| Grok | 4.5 |

### Prefer these workers

| Lead | Delegate mundane work to |
|------|--------------------------|
| Claude Fable / Opus | Claude Sonnet 5 medium (or similar mid-tier Sonnet) |
| Codex / GPT 5.6 Sol or Terra | GPT 5.6 Luna medium |
| Grok 4.5 | Composer 2.5 |

If the exact mid-tier slug is unavailable in the product UI, pick the cheapest
capable model in the same family — do not run the chore on the lead.

### Delegate these (non-exhaustive)

- Running tests (`cargo test`, feature matrices, flaky re-runs)
- Builds / checks (`cargo build`, `cargo check`, clippy)
- Formatting (`cargo fmt`, similar)
- Wide file scraping (bulk `grep`/`rg`, reading many files for inventory)
- Collecting long command output, CI logs, or compile error dumps
- Other high-token, low-judgment chores where the lead only needs a verdict

**Keep on the lead:** architecture choices, GameCommand / ECS design, BRP
semantics, non-obvious debugging, writing the plan, and interpreting worker
summaries into the next change.

### Required worker summary structure

When launching a worker, tell it to return **only** this shape (no raw log
dumps unless a short excerpt is necessary to unblock the lead):

```markdown
## Result
pass | fail | partial

## What ran
- <command or task> → <exit / outcome>

## Key findings
- <1–5 bullets; facts only>

## Failures / warnings
- <none, or short verbatim excerpt + file:line if known>

## Paths
- <files or crates that matter for the next lead step>

## Suggested next step for lead
- <one concrete action>
```

Workers must not paste full test suites, full `cargo` trees, or entire files
back to the lead. Prefer counts, names of failing tests, and the first
actionable error.


The authoritative list lives in `crates/alveus-command/src/lib.rs` (`GameCommand`), fully
documented per-variant in that enum's doc comments. Do not treat line-number
citations elsewhere in this file as stable anchors — search for the type/name.

- **Player verbs**: `Move`, `MoveStop`, `NavigateListMenu`, `Interact`, `DropItem`, `EnterBuilding`,
  `ExitRoom`, `PauseToggle`, `Play`, `Back`, `SkipSplash`, `OpenSettings`,
  `OpenCredits`, `Continue`, `QuitToTitle`.
- **Debug / harness verbs** (mirror in-game debug keys or exist for control —
  avoid these when reproducing genuine play): `ImproveStat`, `WorsenStat`,
  `AdvanceTime`, `AdjustVolume`, `AdvanceFrames`.
- **Observation harness**: `Screenshot` — not a player action, but a legitimate
  agent tool for visual inspection (see §3).

Triggering a verb over BRP:

```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "method": "world.trigger_event",
  "params": {
    "event": "alveus_headless::command::GameCommand",
    "value": { "Move": "Right" }
  }
}
```

Unit-variant verbs serialize as a bare string (`"SkipSplash"`, `"MoveStop"`).
Data-carrying verbs serialize as a tagged object (`{ "Move": "Up" }`,
`{ "Screenshot": { "path": "/abs/path/to/repo/screenshots/frame.png" } }`). Use `registry.schema` to confirm
the exact Reflect shape of struct-like variants rather than guessing.

Commands are **buffered and applied once per frame**, then state transitions run.
Do not assume a verb's effect is visible in the same tick you sent it — send the
verb, advance/await a frame, then read state back.

---

## 3. Observation

**Prefer structured ECS reads** — reach for these first and for most assertions:

| Method | Good for |
|--------|----------|
| `world.query` | Components: player tile, animal stats, interactables, … |
| `world.get_resources` | `State<Screen>`, `State<Menu>`, `Pause`, satchel, upkeep, … |
| `registry.schema` / `rpc.discover` | Learning types, verb shapes, available components |

**Screenshots are a first-class supplement, not something to avoid.** Use
`GameCommand::Screenshot { path }` when:

- Checking for **visual bugs** — layout, sprites, camera framing, missing assets,
  UI overlap, wrong room geometry, HUD/menu/toast placement.
- Logic via queries is inconclusive or painfully indirect, and **seeing the frame**
  gives you another data point (did the interior load? is the player where you
  expect on screen? is the HUD/toast visible?).

Headless captures are the **composed game frame** (world + UI). The offscreen
`Camera2d` is marked `IsDefaultUiCamera`, so root UI (HUD, menus, toasts, room
chrome) renders into the same `HeadlessRenderTarget` that `Screenshot` dumps.
Layout uses `--resolution` / `HeadlessResolution` (no primary window / Xvfb).
Visual PNG assertions still need a wgpu adapter (GPU or lavapipe); keep them out
of plain CI. Prefer ECS queries for **game logic**. A retained smoke driver at
`scripts/headless_ui_screenshot_smoke.py` writes under `screenshots/` and
**fails** if the PNGs lack UI overlay pixels (dark HUD chrome / teal accents, or
pause-menu blue) — a world-only capture regression must not pass.

Workflow: trigger `Screenshot`, wait ~2 frames, then read/analyze the PNG at
`path`. In a Python driver, write under the repo's **`screenshots/`** directory
(e.g. `screenshots/playtest_overview.png` — use an absolute path when triggering
the verb). Create `screenshots/` if needed (`mkdir -p screenshots`). Do not
scatter captures in the repo root or `/tmp/` unless you have a specific reason.
Inspect the image in your session and attach it when reporting to the user. This
rides the normal verb set — no custom BRP observation method.

Use queries as the source of truth for **game logic** (stats, screen state, tile
position). Use screenshots for **presentation** and as a sanity check when
debugging hard flows. Both together are fine.

**Player tile is mandatory after locomotion.** `CurrentTilePosition` is
`#[reflect(Component)]` and queryable at
`alveus_components::CurrentTilePosition` (filter with `Player`). After
every step, read it before deciding the next direction or whether a navigation
goal was reached. If the tile did not change, the step was blocked — pick another
direction or query dynamic obstacle positions rather than blindly repeating the
same move.

---

## 4. The preferred workflow: a Python driver script

**Agents must orchestrate the game through a Python script, not by hand-firing
`curl`/shell commands one at a time.** The script is the agent's working memory of
a session: every step it takes, every coordinate it counts, every assertion it
makes lives in that file. This keeps sessions reproducible and inspectable.

### Why
- The user can ask "show me exactly what you did" and the agent outputs the
  script verbatim.
- Tile-counting plus per-step `CurrentTilePosition` reads keeps navigation
  deterministic and easy to reason about *in code*, but error-prone as scattered
  shell invocations.
- A script is trivially promotable into a Rust e2e test later.

### Rules
- Keep the driver in `scripts/` (e.g. `scripts/headless_<scenario>_demo.py`).
- Write screenshot output to `screenshots/` at the repo root (see §3).
- Use only the stdlib (`urllib.request`, `json`, `time`) — no third-party deps
  required for the Python side. `scripts/headless_nutrition_house_demo.py` is one
  example driver; treat its constants the same way as the table above.
- Track everything the agent cares about (current tile, target tile, expected
  screen) as variables in the script, not in your head — and refresh `current
  tile` from `player_tile()` after every step, never from step count alone.
- The agent should retain this script in its context/artifacts and be able to
  print it on request.
- Do **not** drive the game by issuing many separate shell/`curl` calls. One
  script, run once.
- **Stop the headless server** when the session ends (see §6 gotchas). A forgotten
  background instance will keep mutating `save.ron`.

### Navigation: count tiles to plan, query position to confirm
The player spawns at a known tile and the overview map layout is static, so
counting steps is a fine way to *plan* a route. Don't reach for pathfinding.
But **always query `CurrentTilePosition` after each step** — this is required,
not optional. A held `Move` may end with the player still on the same tile when
blocked by a wall or obstacle. Dynamic obstacles are especially unpredictable:
animals wander inside enclosures, and tiles can become blocked at runtime without
any change to the map file. Your driver script must treat the queried tile as
the only authoritative position; update your tracked `(x, y)` from that read
every time.

**The table below is an illustrative snapshot, not a guarantee.** Values *and*
the file paths / symbol names cited for finding them can change without this doc
being updated — search the repo (grep, `registry.schema`, map assets, runtime
logs) if a pointer is stale. Re-derive coordinates whenever behaviour doesn't
match — **by reading `CurrentTilePosition`**, not by assuming the step count was
correct.

| Fact (example) | Value (at time of writing) | Likely source in repo (verify — may move) |
|----------------|----------------------------|-------------------------------------------|
| Player spawn (overview) | tile `(0, 0)` | `PlayerSpawnPoint::default()` in `crates/alveus-world/src/room.rs` |
| Nutrition House entrance tiles | `x = 32..=35`, `y = 11..=12` | `Inserting snapped TileGroup` log from `crates/alveus-world/src/entrance.rs`, or `assets/maps/overview/map.tmx` entrance objects |
| Push Pop Enclosure entrance tiles | `x = 38..=41`, `y = 11..=12` | same as above |
| Nutrition House exit spawn (overview) | `(33, 12)` | `NutritionHousePlugin` `RoomConfig` in `crates/alveus-world/src/room.rs` |
| Push Pop exit spawn (overview) | `(40, 33)` | `PushPopEnclosurePlugin` `RoomConfig` in `crates/alveus-world/src/room.rs` |
| One tile step (sim time) | ~`PLAYER_MOVE_DURATION_SECS` | `alveus_configs::PLAYER_MOVE_DURATION_SECS` |

### Minimal script skeleton

```python
import json, time, urllib.request
BASE = "http://127.0.0.1:15702/"  # default; see DEFAULT_BRP_PORT in crates/alveus-headless/src/lib.rs
EVENT = "alveus_headless::command::GameCommand"

def rpc(method, params=None):
    body = {"jsonrpc": "2.0", "id": 1, "method": method}
    if params is not None:
        body["params"] = params
    req = urllib.request.Request(BASE, data=json.dumps(body).encode(),
                                 headers={"Content-Type": "application/json"})
    out = json.load(urllib.request.urlopen(req, timeout=30))
    if "error" in out:
        raise RuntimeError(out["error"])
    return out.get("result")

def trigger(value):           # send a GameCommand
    rpc("world.trigger_event", {"event": EVENT, "value": value})

def player_tile():            # authoritative position — call after every step
    res = rpc("world.query", {"data": {
        "components": ["alveus_components::CurrentTilePosition"],
        "has": []},
        "filter": {"with": ["alveus_components::Player"]}})
    row = (res or [None])[0]
    if not row:
        return None
    pos = row["components"]["alveus_components::CurrentTilePosition"]
    inner = pos["0"] if isinstance(pos, dict) and "0" in pos else pos
    return int(inner["x"]), int(inner["y"])

def step(direction, hold=0.35):
    before = player_tile()
    trigger({"Move": direction}); time.sleep(hold); trigger("MoveStop"); time.sleep(0.05)
    after = player_tile()
    if after == before:
        print(f"blocked: still at {after} after Move {direction}")
    return after
```

---

## 5. Testing strategy

There are two complementary layers. **Both matter; add to both.**

### 4a. Unit / integration tests (`MinimalPlugins`)
Fast, headless, no rendering, no networking. Trigger `GameCommand` directly on the
`World` and assert on components/resources. Use the shared harness:

`alveus_app::plugin` is the single production owner of the app-wide `Screen`,
`Menu`, and `Pause` states. Test apps that exercise feature plugins should add it
before those consumers; additional `init_state::<Screen|Menu|Pause>()` calls in
workspace source are intentionally disallowed and covered by the state ownership
regression tests.

```7:18:tests/common/mod.rs
pub fn minimal_stats_app(save_path: &str) -> App { /* alveus_app + MinimalPlugins + StatsPlugin + CommandPlugin */ }
```

Pattern:

```rust
let mut app = common::minimal_stats_app("test_x.ron");
app.world_mut().trigger(GameCommand::Move(MovementIntent::Up));
app.update();
// assert MovementController.intent / Screen / stats ...
common::cleanup_save("test_x.ron");
```

Existing examples: `tests/command_tests.rs`, `tests/stats_tests.rs`,
`tests/interaction_tests.rs`, `tests/reflect_registry_tests.rs`.

### 4b. End-to-end BRP tests (`tests/`, behind `headless`)
Exercise the real protocol path via the in-process `BrpSender` pipeline: push
`BrpMessage`s and run `app.update()`. See `tests/brp_tests.rs` and
`tests/stdio_tests.rs`. This validates the exact `world.trigger_event` /
`world.query` wire format an external client uses, without sockets or timing
flakiness.

Guidelines:
- Gate networked/headless tests with `#![cfg(feature = "headless")]` (see the top
  of `tests/brp_tests.rs`).
- Promote any non-trivial Python driver scenario (§4) into a `tests/` e2e test so
  it can't silently regress.
- Render/screenshot assertions need a real wgpu device (a GPU or software
  Vulkan/lavapipe). Don't make plain CI depend on them; the app is windowless so
  no `Xvfb` is needed, but a GPU adapter still is.

### Running tests
```bash
cargo test --profile ci                    # default features
cargo test --features headless --profile ci  # includes BRP e2e tests
```

---

## 6. Building & running headless

```bash
# Realtime: frames advance on a wall-clock metronome — use this for interactive
# BRP control and for Python driver scripts.
cargo run --features headless -- --headless --realtime --port 15702 --no-stdio

# Step mode: deterministic; the HTTP server blocks until a client sends
# GameCommand::AdvanceFrames(n). Great for reproducible CI, NOT for live driving.
cargo run --features headless -- --headless --step --port 15702 --no-stdio
```

Flags: `--headless`, `--step` / `--realtime`, `--port N`,
`--resolution WxH`, `--no-stdio`. Defined in `src/lib.rs` (`parse_run_mode`).

### Gotchas (read these — they have bitten us)
- **Use `--realtime` to drive live.** In `--step` mode the BRP HTTP server will
  appear to hang because no frames advance until you send `AdvanceFrames`.
- **`cargo run`, not the bare binary.** The dev build uses Bevy dynamic linking;
  invoking `target/debug/alveus-idle-cli` directly fails to find
  `libbevy_dylib*.so`/`libstd*.so`. Launch via `cargo run` so rpaths resolve.
- **Headless windowless mode sets `ExitCondition::DontExit`** (`src/lib.rs`),
  otherwise Bevy quits immediately with "No windows are open, exiting". Keep that.
- **`--features headless` is required** for `--headless` to do anything; without
  it the runtime flag panics by design.
- Screenshots are written asynchronously — wait ~2 frames before reading the PNG.
- **Stop the headless server when you're done.** A background `cargo run --features
  headless -- …` keeps simulating in realtime and **autosaves to `save.ron` every
  ~5 seconds** (default [`SavePath`] in `crates/alveus-stats/src/lib.rs`). Kill the process after
  your driver script finishes — do not leave it running in a background terminal.
  Verify with `pgrep -af alveus-idle-cli` (no matches) or that `save.ron`'s mtime
  is no longer changing.

---

## 7. Architecture conventions to uphold

- **Extract-and-route, no duplication.** Keyboard readers and BRP clients must
  `trigger(GameCommand::…)`; `CommandPlugin` routes each variant to private or
  feature-owned request observers (e.g. `InteractionRequest`, `RoomRequest`) that
  call the same helpers UI code uses (`perform_drop`, `open_care_menu`,
  `try_enter_room`, `advance_simulated_hours`). Keyboard handlers stay thin.
  Never fake key input to reach a verb.
- **Observer routing + deferred phase flush.** `enqueue_game_command` only buffers
  into `DeferredGameCommands`. `route_deferred_game_commands` runs in `First`,
  `PostUpdate`, and (with `remote`) after `RemoteSystems::ProcessRequests` in
  `RemoteLast`, validating FatalError and triggering internal request events.
  That preserves the PreUpdate-input contract: local verbs do not mutate
  gameplay/`NextState` before `Update`. After each nonempty route batch,
  `PendingCommandStateFlush` queues `StateTransition` once via
  `Commands::run_schedule`. Internal request events are **not**
  Reflect-registered. Preserve this ordering when touching
  `crates/alveus-command/src/lib.rs`.
- **Reflect everything observable/triggerable.** New components/resources/events
  that an agent must query or trigger need `#[derive(Reflect)]`,
  `#[reflect(Component/Resource/Event)]`, and registration in
  `register_types` (`crates/alveus-reflect/src/lib.rs`). If it's not registered,
  `world.query`/`registry.schema`/`world.trigger_event` can't see it.
- **Observation stays client-side.** Derived facts (adjacency, joins like animal
  cleanliness via enclosure) are computed by the client/script/test from raw query
  results — do not add server-side snapshot resources that can drift.

---

## 8. Definition of done (per change)

- [ ] `cargo build --features headless` is clean (no new warnings you introduced).
- [ ] `cargo test --profile ci` and `cargo test --features headless --profile ci`
      pass.
- [ ] New agent-facing types are `Reflect`-registered (§7) if they must be
      observed/triggered.
- [ ] New/changed verbs have accurate doc comments in `GameCommand`.
- [ ] If you validated behavior with a Python driver, the key assertions are also
      captured in a Rust test (§5), and the script is saved under `scripts/`.
- [ ] No new custom BRP methods, no bespoke observation struct, no key-injection
      hatch, no auto-navigation verbs.
- [ ] Any headless `cargo run` started for manual/BRP playtesting was stopped
      afterward (no stray process still autosaving `save.ron`).

---

## 9. Quick reference

| Need | Use |
|------|-----|
| Do something in the game | `world.trigger_event` → `GameCommand` |
| Read player position (after every move) | `world.query` `CurrentTilePosition` with `Player` filter |
| Read current screen/menu/pause | `world.get_resources` `State<Screen>` etc. |
| Discover types / verb shapes | `registry.schema`, `rpc.discover` |
| Visual check / visual bugs | `GameCommand::Screenshot { path }` → `screenshots/*.png`, then inspect |
| Fast, logic-only test | `MinimalPlugins` unit test (`tests/common`) |
| Protocol-level test | in-process `BrpSender` e2e (`tests/brp_tests.rs`) |
| Live exploration/debugging | Python driver in `scripts/` against `--realtime` |
| Mundane builds/tests/scrapes on a costly lead | Delegate to a mid-tier worker; require the fixed summary (Cost-tier delegation) |

BRP event path: `alveus_headless::command::GameCommand` (defined in
`crates/alveus-command/src/lib.rs` with a compatibility type path). Default port: `15702` (`DEFAULT_BRP_PORT` in
`crates/alveus-headless/src/lib.rs`) unless overridden with `--port`.
