# Alveus Sanctuary Keeping — Design Specification

> **Status:** Living document · **Engine:** Bevy 0.18 · **Tile Size:** 32×32 px  
> **Source of truth for all gameplay systems, room layouts, stat formulas, and data schemas.**

This document replaces `concept.md` as the **implementation reference**. Every value, coordinate, formula, and behavior described here is deterministic and machine-verifiable. Companion data files in `design/data/` and schemas in `design/schemas/` provide the canonical definitions that game code must conform to.

---

## Table of Contents

1. [Architecture Overview](#1-architecture-overview)
2. [Coordinate System & Grid](#2-coordinate-system--grid)
3. [Screen State Machine](#3-screen-state-machine)
4. [Overview Map](#4-overview-map)
5. [Room System](#5-room-system)
6. [Room Layouts](#6-room-layouts)
7. [Animal Ambassadors](#7-animal-ambassadors)
8. [Stat Decay & Upkeep](#8-stat-decay--upkeep)
9. [Inventory (Caretaker Satchel)](#9-inventory-caretaker-satchel)
10. [Interaction System](#10-interaction-system)
11. [Passive Economy](#11-passive-economy)
12. [Caretaker Team](#12-caretaker-team)
13. [Daily Events](#13-daily-events)
14. [Collectibles & Progression](#14-collectibles--progression)
15. [Neglect Freeze](#15-neglect-freeze)
16. [Education System](#16-education-system)
17. [Data File Index](#17-data-file-index)

---

## 1. Architecture Overview

### Tech Stack

| Layer | Technology | Version |
|---|---|---|
| Engine | Bevy | 0.18 |
| Tiled Integration | bevy_ecs_tiled | 0.11.2 |
| UI Animation | bevy_tweening | 0.15 |
| RNG | rand | 0.9 |
| Language | Rust | 2024 edition |

### Data-Driven Design

All game content is defined in JSON data files validated against JSON Schemas. These values should be further canonicalized and restuctured if needed to match in-code structs. They could be rewritten and loaded via RON https://docs.rs/bevy_ron/latest/bevy_ron/.

```
design/
├── design.md                   ← This document
├── schemas/
│   ├── room.schema.json        ← Room layout structure
│   ├── animal.schema.json      ← Animal ambassador definitions
│   ├── item.schema.json        ← Inventory item definitions
│   ├── economy.schema.json     ← Economy & shop configuration
│   ├── overview_map.schema.json← Overview map structure
│   └── event.schema.json       ← Daily event definitions
├── data/
│   ├── animals.json            ← All animal ambassadors
│   ├── items.json              ← All inventory items
│   ├── economy.json            ← Coin tiers, caretakers, stamps, decor
│   ├── events.json             ← Daily event pool
│   └── overview_map.json       ← Building placements on overview
└── rooms/
    ├── nutrition_house.json    ← Nutrition House room layout
    ├── studio.json             ← The Studio room layout
    ├── pasture.json            ← The Pasture room layout
    └── hq_office.json          ← HQ Office room layout
```

---

## 2. Coordinate System & Grid

### Constants

| Constant | Value | Location |
|---|---|---|
| `TILE_SIZE` | `32` (pixels) | `src/demo/level.rs` |
| `GRID_SNAP_EPSILON` | `0.05` (pixels) | `src/demo/entrance.rs` |
| `PLAYER_Z_INDEX` | `2.0` | `src/demo/player.rs` |

### Coordinate Convention

- **Origin:** Bottom-left corner of the map (Bevy's default 2D coordinate system).
- **Axes:** X increases rightward, Y increases upward.
- **Data Type:** `TilePosition { x: u32, y: u32 }` — unsigned, so coordinates cannot be negative.
- **World-to-Tile Conversion:** `world_position = tile_position * TILE_SIZE`
- **Tile-to-World Conversion:** `tile_position = (world_position / TILE_SIZE).round()`

### Grid-to-Pixel Mapping

For a tile at position `(tx, ty)`:
```
pixel_x = tx * 32
pixel_y = ty * 32
```

The Tiled map is offset by `(-TILE_SIZE/2, -TILE_SIZE/2, 0)` = `(-16, -16, 0)` so that tile sprite centers align with grid intersection points.

### Z-Layer Ordering

Deterministic render ordering via z-coordinates:

| Layer | Z-Value | Contents |
|---|---|---|
| Floor tiles | `0.0` | Interior floor |
| Door tile | `0.01` | Room exit door |
| Walls | `0.1` | Perimeter walls |
| Objects | `0.2` | Furniture, fixtures |
| Fences/Zones | `0.15` | Playpen fences |
| Animals | `1.0` | NPC animal sprites |
| Player | `2.0` | Player character |
| UI overlays | `10.0+` | HUD, menus, toasts |

---

## 3. Screen State Machine

```
┌──────────┐    ┌──────────┐    ┌──────────┐    ┌──────────────┐
│  Splash  │───▶│  Title   │───▶│ Loading  │───▶│   Gameplay   │
└──────────┘    └──────────┘    └──────────┘    │  (Overview)  │
                                                └──────┬───────┘
                                                       │ Enter
                                                       ▼
                                          ┌─────────────────────────┐
                                          │     InRoom(variant)     │
                                          │  NutritionHouse         │
                                          │  Studio                 │
                                          │  Pasture                │
                                          │  HqOffice               │
                                          └─────────────────────────┘
                                                       │ Backspace / Exit Door
                                                       ▼
                                                ┌──────────────┐
                                                │   Gameplay   │
                                                │  (Overview)  │
                                                └──────────────┘
```

### State Definitions (Rust)

```rust
#[derive(Clone, Copy, Eq, PartialEq, Hash, Debug)]
pub enum InRoom {
    NutritionHouse,
    Studio,
    Pasture,
    HqOffice,
}

#[derive(States, Copy, Clone, Eq, PartialEq, Hash, Debug, Default)]
pub enum Screen {
    #[default]
    Splash,
    Title,
    Loading,
    Gameplay,
    InRoom(InRoom),
}
```

### Transition Rules

| From | Trigger | To | Side Effects |
|---|---|---|---|
| Splash | Timer (2s) | Title | — |
| Title | Any key | Loading | Begin asset loading |
| Loading | Assets ready | Gameplay | Spawn overview map |
| Gameplay | Enter key + on entrance zone | InRoom(X) | Dismiss toast, spawn room interior |
| InRoom(X) | Backspace key OR walk to exit door | Gameplay | Set `PlayerSpawnPoint` to `exit_target`, despawn room |

---

## 4. Overview Map

**Schema:** [`schemas/overview_map.schema.json`](schemas/overview_map.schema.json)  
**Data:** [`data/overview_map.json`](data/overview_map.json)

### Terrain

- **Grid Size:** 50×50 tiles (1600×1600 pixels)
- **Tile Size:** 32×32 pixels
- **Terrain Types:**
  | Tile ID | Name | Asset | Walkable |
  |---|---|---|---|
  | 0 | Sand Path | `sand_tile.png` | Yes |
  | 1 | Sand-Grass Transition | `sand_grass_tile.png` | Yes |
  | 2 | Grass | `grass_tile.png` | Yes |

- **Map Format:** Tiled `.tmx` with infinite chunks (16×16 tile chunks)
- **Map Offset:** The `TiledMap` entity is translated by `(-16, -16, 0)` so tile centers align.

### Building Placements

Each building has an exterior sprite placed on the overview map and a rectangular entrance zone that triggers the room transition:

| Building | Overview Position | Entrance Zone (tiles) | Target Room |
|---|---|---|---|
| Nutrition House | (31, 14) | (32,13)→(35,14) | `nutrition_house` |
| The Studio | (12, 12) | (14,13)→(17,14) | `studio` |
| The Pasture | (16, 22) | (19,23)→(22,24) | `pasture` |
| HQ Office | (22, 6) | (24,8)→(27,9) | `hq_office` |

### Player Initial Spawn

On first game launch (new save), the player spawns at tile `(25, 10)` — outside the HQ Office entrance.

### Entrance Detection

The entrance system (`src/demo/entrance.rs`) works as follows:

1. **Tiled objects** with `BuildingEntrance` properties are loaded by `bevy_ecs_tiled`.
2. On `Added<BuildingEntrance>`, the `validate_and_snap_entrances` system:
   - Verifies the object position is grid-aligned (within `GRID_SNAP_EPSILON = 0.05px`).
   - Verifies dimensions are exact multiples of `TILE_SIZE`.
   - Converts the Tiled world-space rectangle to a `TileGroup::Rectangle` with `bottom_left` / `top_right` coordinates.
   - **Panics** on misalignment (map integrity enforcement).
3. Every frame, `check_player_entrance_transitions` checks if the player's `CurrentTilePosition` overlaps any entrance `TileGroup`. On overlap:
   - Inserts `BuildingEntrance` component on the player entity.
   - Triggers `PlayerEnteredBuildingEvent` → Toast system shows "Press [Enter] to enter {building}".
   - On leaving the zone: removes `BuildingEntrance` component, triggers `PlayerExitedBuildingEvent` → Dismisses toast.

---

## 5. Room System

**Schema:** [`schemas/room.schema.json`](schemas/room.schema.json)

### Room Lifecycle

Rooms are self-contained game states. The `build_room<S>` function in `src/demo/room.rs` generically wires up any room via `RoomConfig<S>`:

```rust
pub struct RoomConfig<S: States + FreelyMutableState> {
    pub room_state: S,           // e.g., Screen::InRoom(InRoom::NutritionHouse)
    pub gameplay_state: S,       // Screen::Gameplay
    pub entrance: BuildingEntrance,
    pub room_spawn: TilePosition,
    pub exit_spawn: TilePosition,
    pub exit_door: TilePosition,
    pub spawn_interior_fn: fn(&mut ChildSpawnerCommands, &mut Assets<Mesh>, &mut Assets<ColorMaterial>),
    pub room_title: String,
}
```

**OnEnter(room_state):**
1. Spawn parent entity with `DespawnOnExit(room_state)`.
2. Spawn player at `room_spawn` with `CurrentTilePosition` and `DesiredTilePosition`.
3. Call `spawn_interior_fn` to build floors, walls, objects, zones.
4. Spawn UI overlay: room title (32px, green) + "Press [Backspace] to exit" hint (18px, grey).
5. Dismiss any active toast.

**Exit Conditions (polled every frame in room state):**
- `Backspace` pressed → exit immediately.
- Player walks to `exit_door` tile → exit immediately.
- On exit: set `PlayerSpawnPoint.position = exit_spawn`, transition to `gameplay_state`.

### Room Interior Construction

Rooms are currently procedurally built with colored rectangles (not Tiled maps). Each room is defined by its JSON data file and constructed as follows:

1. **Floor:** For `style: "fill_interior"`, spawn `Mesh2d(Rectangle 30×30)` tiles at every position `(x, y)` where `1 ≤ x ≤ width-2` and `1 ≤ y ≤ height-2`.
2. **Walls:** For `style: "perimeter"`, spawn wall tiles with `Obstacle` component at all perimeter positions, excluding tiles listed in `excluded_tiles` (i.e., the door).
3. **Door:** A single walkable tile at `door.position` with a distinct color. Stepping on it triggers room exit.
4. **Objects:** Each object from the `objects` array becomes an entity with:
   - `Name`, `Mesh2d`, `MeshMaterial2d`, `Transform` (at tile position × TILE_SIZE), `TilePosition`.
   - `Obstacle` component if `is_obstacle = true`.
   - (Future) `Interactable` component if `is_interactable = true`.
5. **Zones:** Fence obstacles on specified `fence_sides`, with a walkable gap at `gate_position`.
6. **Dynamic Spawns:** On each daily reset, spawn `count.min` to `count.max` instances at random free tiles within `spawn_area`.

---

## 6. Room Layouts

Each room layout is fully specified in its JSON data file. The following sections provide human-readable grid maps and summaries.

### 6.1 Nutrition House

**Data:** [`rooms/nutrition_house.json`](rooms/nutrition_house.json)

| Property | Value |
|---|---|
| Grid Size | 11×11 tiles |
| Interior Area | 9×9 tiles (1,1)→(9,9) |
| Door | (5, 0) south wall |
| Spawn Point | (5, 2) |
| Exit Target | (33, 12) on overview |

**Grid Map** (Y=10 at top, Y=0 at bottom):
```
Y10: [W][W][W][W][W][W][W][W][W][W][W]
 Y9: [W][ ][ ][ ][ ][ ][ ][ ][ ][ ][W]
 Y8: [W][ ][F][ ][ ][ ][ ][C][ ][ ][W]   F=Diet Fridge  C=Smoothie Blender
 Y7: [W][ ][ ][ ][P][P][P][ ][ ][ ][W]   P=Prep Table (3 tiles)
 Y6: [W][ ][ ][ ][ ][ ][ ][ ][ ][ ][W]
 Y5: [W][ ][S][ ][ ][ ][ ][f][E][f][W]   S=Seed Chest  E=Enrichment Post  f=fence
 Y4: [W][ ][ ][ ][ ][ ][ ][f][·][f][W]   ·=Polly home pos (walkable inside pen)
 Y3: [W][ ][ ][ ][ ][ ][ ][g][B][f][W]   g=Gate(walkable)  B=Feed Bowl
 Y2: [W][ ][ ][ ][ ][↑][ ][f][N][f][W]   ↑=Player Spawn  N=Nesting Box
 Y1: [W][ ][ ][ ][ ][ ][ ][f][f][f][W]   f=playpen fence
 Y0: [W][W][W][W][W][D][W][W][W][W][W]   D=Door
      X0 X1 X2 X3 X4 X5 X6 X7 X8 X9 X10
```

**Objects:**

| ID | Name | Position | Obstacle | Interactable | Interaction |
|---|---|---|---|---|---|
| `diet_fridge` | Diet Fridge | (2,8) | ✓ | ✓ | `open_menu` → fridge_menu |
| `prep_table` | Prep Table | (4-6,7) | ✓ | ✓ | `mini_chore` → chop_veggies |
| `smoothie_blender` | Smoothie Blender | (7,8) | ✓ | ✓ | `mini_chore` → blend_smoothie |
| `seed_chest` | Seed Chest | (2,5) | ✓ | ✓ | `give_item` → chicken_grains |
| `polly_feed_bowl` | Polly's Feed Bowl | (8,3) | ✓ | ✓ | `feed_animal` → polly (hunger) |
| `polly_enrichment_post` | Polly's Enrichment Zone | (8,5) | ✓ | ✓ | `enrich_animal` → polly (happiness) |
| `polly_nesting_box` | Polly's Nesting Box | (8,2) | ✓ | ✗ | — |

**Zones:**

| Zone | Bounds | Fence Sides | Gate |
|---|---|---|---|
| `polly_playpen` | (7,1)→(9,5) | north, west | (7,3) |

**Resident Animals:** Polly (Silkie Chicken), home position (8,4), wanders within playpen.

---

### 6.2 The Studio

**Data:** [`rooms/studio.json`](rooms/studio.json)

| Property | Value |
|---|---|
| Grid Size | 15×15 tiles |
| Interior Area | 13×13 tiles (1,1)→(13,13) |
| Door | (7, 0) south wall |
| Spawn Point | (7, 2) |
| Exit Target | (15, 14) on overview |

**Grid Map** (Y=14 at top, Y=0 at bottom):
```
Y14: [W][W][W][W][W][W][W][W][W][W][W][W][W][W][W]
Y13: [W][ ][ ][ ][ ][ ][ ][ ][ ][ ][ ][ ][ ][ ][W]
Y12: [W][ ][ ][G][G][G][ ][ ][ ][ ][ ][S][S][S][W]   G=Georgie Tank  S=Siren Enclosure
Y11: [W][ ][ ][ ][ ][ ][ ][ ][ ][ ][ ][ ][ ][ ][W]
Y10: [W][ ][ ][T][ ][ ][ ][ ][ ][ ][ ][B][ ][ ][W]   T=Tank Water  B=Climbing Branches
 Y9: [W][ ][ ][ ][ ][ ][ ][ ][ ][ ][ ][ ][ ][ ][W]
 Y8: [W][ ][ ][ ][ ][ ][ ][ ][ ][ ][ ][ ][ ][ ][W]
 Y7: [W][ ][ ][K][K][ ][ ][L][L][ ][ ][ ][ ][ ][W]   K=Studio Desk  L=Livestream Soundboard
 Y6: [W][ ][ ][ ][ ][ ][ ][ ][ ][ ][ ][ ][ ][ ][W]
 Y5: [W][ ][ ][ ][ ][ ][ ][ ][ ][ ][ ][ ][ ][ ][W]
 Y4: [W][ ][ ][ ][ ][ ][ ][ ][ ][ ][ ][ ][ ][ ][W]
 Y3: [W][ ][ ][ ][ ][ ][ ][ ][ ][ ][ ][ ][ ][ ][W]
 Y2: [W][ ][ ][ ][ ][ ][ ][↑][ ][ ][ ][ ][ ][J][W]   ↑=Spawn  J=Bug Jar Rack
 Y1: [W][ ][ ][ ][ ][ ][ ][ ][ ][ ][ ][ ][ ][ ][W]
 Y0: [W][W][W][W][W][W][W][D][W][W][W][W][W][W][W]   D=Door
      X0 X1 X2 X3 X4 X5 X6 X7 X8 X9 10 11 12 13 14
```

**Objects:**

| ID | Name | Position | Size | Obstacle | Interactable | Interaction |
|---|---|---|---|---|---|---|
| `georgie_tank` | Georgie's Glass Tank | (3,12) | 3×2 | ✓ | ✓ | `feed_animal` → georgie |
| `siren_enclosure` | Siren's Warm Enclosure | (11,12) | 3×2 | ✓ | ✓ | `mini_chore` → mist |
| `livestream_soundboard` | Livestream Soundboard | (7,7) | 2×1 | ✓ | ✓ | `toggle_fixture` → +50% happiness |
| `bug_jar_rack` | Bug Jar Rack | (13,2) | 1×1 | ✓ | ✓ | `give_item` → cricket_box |
| `studio_desk` | Studio Desk | (3,7) | 2×1 | ✓ | ✗ | — |
| `georgie_tank_clean` | Tank Water | (3,10) | 1×1 | ✓ | ✓ | `clean_tile` → georgie cleanliness |
| `siren_enrichment_branch` | Climbing Branches | (11,10) | 1×1 | ✓ | ✓ | `enrich_animal` → siren happiness |

**Resident Animals:** Georgie (African Bullfrog) at (4,12), Siren (Ball Python) at (12,12).

---

### 6.3 The Pasture

**Data:** [`rooms/pasture.json`](rooms/pasture.json)

| Property | Value |
|---|---|
| Grid Size | 21×21 tiles |
| Interior Area | 19×19 tiles (1,1)→(19,19) |
| Door | (10, 0) south wall |
| Spawn Point | (10, 2) |
| Exit Target | (20, 24) on overview |

**Grid Map** (simplified — 21×21 is large):
```
Y20: [W][W][W][W][W][W][W][W][W][W][W][W][W][W][W][W][W][W][W][W][W]
Y19: [W][ grass area                                              ][W]
Y18: [W]                                                           [W]
Y17: [W]   [SHELTER]                                               [W]   Shelter at (4,17) 3×2
Y16: [W]   [SHELTER]                                               [W]
Y15: [W]                              [F]                          [W]   F=Feed Bin at (15,15)
 ...
Y10: [W][V]                [T][T]                                  [W]   V=Spigot(1,10) T=Trough(10-11,10)
 ...
 Y8: [W]                                             [M]           [W]   M=Mirror Post at (18,8)
 ...
 Y2: [W]                             [↑]                           [W]   ↑=Spawn at (10,2)
 Y1: [W]                                                           [W]
 Y0: [W][W][W][W][W][W][W][W][W][W][D][W][W][W][W][W][W][W][W][W][W]   D=Door
```

**Dynamic Spawns:**
- **Manure Piles:** 2-4 spawned per daily reset at random free grass tiles within (2,2)→(19,19). Each cleaned pile restores `0.34` cleanliness (≈3 piles for full restoration).

**Resident Animals:** Stompy (Emu) at (10,14), wanders across entire pasture interior.

---

### 6.4 HQ Office

**Data:** [`rooms/hq_office.json`](rooms/hq_office.json)

| Property | Value |
|---|---|
| Grid Size | 11×11 tiles |
| Interior Area | 9×9 tiles (1,1)→(9,9) |
| Door | (5, 0) south wall |
| Spawn Point | (5, 2) |
| Exit Target | (25, 9) on overview |

**Grid Map:**
```
Y10: [W][W][W][W][W][W][W][W][W][W][W]
 Y9: [W][ ][ ][ ][ ][L][ ][ ][ ][ ][W]   L=Donation Link Sign
 Y8: [W][ ][R][ ][ ][ ][ ][ ][A][ ][W]   R=Staff Roster  A=Stamp Album Desk
 Y7: [W][ ][ ][ ][ ][C][C][ ][ ][ ][W]   C=Clipboard (2 tiles)
 Y6: [W][ ][ ][ ][ ][ ][ ][ ][ ][ ][W]
 Y5: [W][K][ ][ ][ ][ ][ ][ ][ ][ ][W]   K=Bookshelf (1×2 at (1,5-6))
 Y4: [W][K][ ][ ][ ][ ][ ][ ][ ][ ][W]
 Y3: [W][ ][ ][ ][ ][ ][ ][ ][O][O][W]   O=Office Couch (2×1)
 Y2: [W][ ][ ][ ][ ][↑][ ][ ][ ][ ][W]   ↑=Player Spawn
 Y1: [W][ ][ ][ ][ ][ ][ ][ ][ ][ ][W]
 Y0: [W][W][W][W][W][D][W][W][W][W][W]   D=Door
```

**Resident Animals:** None.

---

## 7. Animal Ambassadors

**Schema:** [`schemas/animal.schema.json`](schemas/animal.schema.json)  
**Data:** [`data/animals.json`](data/animals.json)

### Roster

| ID | Name | Species | Room | Category |
|---|---|---|---|---|
| `polly` | Polly | Silkie Chicken (*Gallus gallus domesticus*) | Nutrition House | bird |
| `stompy` | Stompy | Emu (*Dromaius novaehollandiae*) | Pasture | bird |
| `georgie` | Georgie | African Bullfrog (*Pyxicephalus adspersus*) | Studio | amphibian |
| `siren` | Siren | Ball Python (*Python regius*) | Studio | reptile |

### Behavior

- Animals are NPC entities with `home_position` and `wander_zone`.
- When idle, animals execute a random walk within their `wander_zone` rectangle.
- Animals respond to care actions with specific animations (`eat`, `happy`).
- When stats are low, animals display the `sad` animation and emit scribble-like emotes.
- **Animals never die, get sick, or face harm.** Neglect only causes visual/stat degradation.

---

## 8. Stat Decay & Upkeep

### Per-Animal Stats

Each animal has three stats normalized to the range `[0.0, 1.0]`:

| Stat | Decay Rate | Time to Empty | Restored By |
|---|---|---|---|
| **Hunger** | `0.04 / hour` | 25.0 hours | Feeding chore |
| **Cleanliness** | `0.03 / hour` | 33.33 hours | Cleaning chore |
| **Happiness** | `0.05 / hour` | 20.0 hours | Enrichment chore |

### Decay Formula

Stats decay continuously in real time (including while the game is closed):

```
current_value = max(0.0, saved_value - (decay_rate * hours_elapsed))
```

Where `hours_elapsed` is computed from the difference between the current UTC timestamp and the last save timestamp.

### Sanctuary Upkeep Score

```
upkeep = (mean_hunger + mean_cleanliness + mean_happiness) / 3.0
```

Where each `mean_*` is the arithmetic mean of that stat across all active (unlocked) animals.

---

## 9. Inventory (Caretaker Satchel)

**Schema:** [`schemas/item.schema.json`](schemas/item.schema.json)  
**Data:** [`data/items.json`](data/items.json)

### Satchel Rules

| Property | Value |
|---|---|
| Max Slots | 2 |
| Stack Support | Per-item (default: non-stackable) |

### Item Lifecycle

1. **Acquisition:** Player interacts with a `give_item` object → item added to first empty satchel slot.
2. **Preparation (optional):** If the item has a `preparation` recipe, the player must carry it to the specified station and complete the mini-chore. The raw item is replaced by the `output_item_id`.
3. **Consumption:** Player interacts with a `feed_animal` or `enrich_animal` object while carrying the `required_item_id` → item consumed, stat restored.

### Item Catalog

| ID | Name | Category | Consumed By | Requires Prep |
|---|---|---|---|---|
| `raw_veggie_tub` | Lettuce & Veggie Tub | raw_diet | — | Yes → `prepared_veggie_diet` |
| `prepared_veggie_diet` | Prepared Veggie Diet | prepared_diet | Stompy | No |
| `chicken_grains` | Chicken Grains | prepared_diet | Polly | No |
| `cricket_box` | Cricket Box | prepared_diet | Georgie | No |
| `carnivore_raw_prep` | Carnivore Raw Prep | raw_diet | — | Yes → `prepared_carnivore_diet` |
| `prepared_carnivore_diet` | Prepared Carnivore Diet | prepared_diet | Siren | No |
| `mini_mirror` | Mini Mirror | enrichment_toy | Polly | No |
| `shiny_object` | Shiny Object | enrichment_toy | Stompy | No |
| `shed_snake_skin` | Shed Snake Skin | collectible | — | No |

### Preparation Recipes

| Input Item | Station | Chore Type | Params | Output Item |
|---|---|---|---|---|
| `raw_veggie_tub` | `prep_table` | tapping | 5 taps | `prepared_veggie_diet` |
| `carnivore_raw_prep` | `prep_table` | instant | — | `prepared_carnivore_diet` |

---

## 10. Interaction System

### Interaction Model

The player interacts with adjacent objects by pressing `Space`. The system:

1. Determines the tile the player is facing (based on last movement direction).
2. Queries all entities with `TilePosition` matching the faced tile AND `is_interactable = true`.
3. Dispatches the interaction by `type`.

### Interaction Types

| Type | Behavior | Example |
|---|---|---|
| `open_menu` | Opens a full-screen overlay menu identified by `menu_id`. | Diet Fridge → fridge_menu |
| `mini_chore` | Triggers a micro-game (tapping, rhythm, etc.) identified by `chore_id`. | Prep Table → chop_veggies (5 taps) |
| `give_item` | Adds `item_id` to the satchel. Fails if satchel is full. | Seed Chest → chicken_grains |
| `place_item` | Consumes `required_item_id` from satchel and places it at the object. | — |
| `feed_animal` | Requires `required_item_id` in satchel. Consumes item, restores `stat_delta` of `stat_affected` on `target_animal_id`. | Feed Bowl + chicken_grains → Polly hunger +1.0 |
| `enrich_animal` | Restores `stat_delta` of `happiness` on `target_animal_id`. May or may not require an item. | Enrichment Post → Polly happiness +1.0 |
| `clean_tile` | Restores `stat_delta` of `cleanliness` on `target_animal_id`. For dynamic spawns, despawns the cleaned object. | Manure pile → Stompy cleanliness +0.34 |
| `toggle_fixture` | Toggles a room fixture on/off. Affects stats of all animals in the room. | Soundboard → +50% happiness (Georgie + Siren) |
| `inspect` | Opens a read-only info panel (e.g., sanctuary status clipboard). | Clipboard → status report |
| `open_shop` | Opens a purchasable catalog menu. | Stamp Desk → stamp album shop |
| `external_link` | Opens an external URL in the system browser. | Donation sign → alveussanctuary.org/donate |

### Interaction Adjacency

The player must be on a tile **cardinally adjacent** to the target object and **facing** it. "Facing" is determined by the player's last `MovementIntent`:

| Last Move | Faced Tile |
|---|---|
| Up | (player.x, player.y + 1) |
| Down | (player.x, player.y - 1) |
| Left | (player.x - 1, player.y) |
| Right | (player.x + 1, player.y) |

For multi-tile objects (e.g., Prep Table spanning (4,7)→(6,7)), the player can interact from any adjacent tile to any tile of the object.

---

## 11. Passive Economy

**Schema:** [`schemas/economy.schema.json`](schemas/economy.schema.json)  
**Data:** [`data/economy.json`](data/economy.json)

### Coin Generation Tiers

Coins accumulate passively (including while offline) based on the Sanctuary Upkeep Score:

| Tier | Min Upkeep | Coins/Hour |
|---|---|---|
| Excellent | ≥ 0.80 | 20 |
| Fair | ≥ 0.30 | 10 |
| Neglected | < 0.30 | 0 |

### Offline Accumulation Formula

```
coins_earned = Σ (coins_per_hour_for_tier * hours_in_tier)
```

Since stats decay continuously, the upkeep score may cross tier boundaries during offline time. The exact calculation must integrate over the decay curve:

1. Project the upkeep score forward using `upkeep(t) = upkeep(t₀) - decay_rate_avg * t`.
2. Find the time points where `upkeep(t)` crosses 0.80 and 0.30 thresholds.
3. Sum `coins_per_hour × hours_spent` in each tier.

### Daily Bonus

Reward: **50 coins** when all animal stats are at 100% (checked via HQ Clipboard interaction).

---

## 12. Caretaker Team

**Data:** [`data/economy.json`](data/economy.json) → `caretakers` array

| ID | Name | Color (sRGB) | Cost | Perk |
|---|---|---|---|---|
| `maya` | Maya Higa | (0.18, 0.80, 0.44) | Free | **Founder's Focus:** 25% faster feeding/cleaning in Pasture |
| `kayla` | Kayla | (0.40, 0.85, 0.70) | 150 | **Reptile Whisperer:** 50% more happiness from Studio enrichment, 15% faster movement in enclosures |
| `connor` | Connor | (0.90, 0.72, 0.15) | 250 | **Enclosure Master:** 2× cleaning speed everywhere |

### Perk System

Perks modify gameplay through typed effects:

| Effect Type | Meaning | Example |
|---|---|---|
| `chore_speed_multiplier` | Multiplies the chore timer duration. < 1.0 = faster. | Maya: 0.75× in Pasture |
| `stat_multiplier` | Multiplies the stat delta from an interaction. > 1.0 = more effective. | Kayla: 1.5× happiness in Studio |
| `speed_multiplier` | Multiplies movement speed. > 1.0 = faster. | Kayla: 1.15× in enclosures |

### Player Rendering

Currently a **colored circle** (`Circle::new(16.)`) using the caretaker's `circle_color`. The sprite system is stubbed but disabled. When sprite assets are available, the player will render as a 32×32 animated sprite sheet with idle/walk animations.

---

## 13. Daily Events

**Schema:** [`schemas/event.schema.json`](schemas/event.schema.json)  
**Data:** [`data/events.json`](data/events.json)

### Event Selection

On each daily reset (first session start of the day), one event is selected using weighted random:

| Event | Weight | Probability |
|---|---|---|
| Normal Day | 0.30 | 30% |
| Rainy Day | 0.20 | 20% |
| Volunteer Day | 0.20 | 20% |
| Siren's Shed Day | 0.15 | 15% |
| Escaped Crickets | 0.15 | 15% |

Selection algorithm:
```
total_weight = sum(event.weight for all events)
roll = rand::random::<f64>() * total_weight
cumulative = 0.0
for event in events:
    cumulative += event.weight
    if roll < cumulative:
        return event
```

### Event Modifications

Events modify the baseline gameplay by:

| Modification Type | Effect |
|---|---|
| `disable_object` | Object becomes non-interactable for the day. |
| `replace_interaction` | Object's normal interaction is swapped for an alternate. |
| `modify_chore_count` | A chore that normally requires 1 interaction now requires N. |
| `spawn_npcs` | Helper NPC entities appear and auto-complete one chore. |
| `add_extra_interactions` | Additional interaction steps are required. |

---

## 14. Collectibles & Progression

### Stamp Album

The Stamp Album is accessed via the `stamp_album_desk` object in the HQ Office. Stamps are purchased with Alveus Coins:

| Stamp | Cost |
|---|---|
| Stompy's Great Escape | 100 |
| Leaf Blower Duel | 150 |
| Georgie's Crown | 200 |
| Python Scarf | 250 |
| Mico's Rant | 300 |

### HQ Customizations

Decorative items purchasable from the Stamp Album Desk:

| Item | Cost | Effect |
|---|---|---|
| Emu Print Rug | 75 | Floor decoration in HQ |
| Plushie Shelf | 150 | Wall decoration in HQ |
| Ambient Soundbox | 200 | Interactive sound effect on step |
| Sanctuary Plaque | 500 | Wall decoration (prestige item) |

### Total Coin Sink

| Category | Total Cost |
|---|---|
| Caretakers (Kayla + Connor) | 400 |
| All Stamps | 1,000 |
| All HQ Customizations | 925 |
| **Grand Total** | **2,325** |

At maximum coin rate (20/hr), earning 2,325 coins takes approximately **116 hours** or **~12 days** of fully maintained sanctuary.

---

## 15. Neglect Freeze

Triggered when **Upkeep Score < 0.30** (typically after ~2 days without playing):

| Effect | Implementation |
|---|---|
| **Visual Desaturation** | Apply a grayscale color grading post-process effect to the camera. |
| **Music Shift** | Crossfade background music to a minor-key piano/acoustic track. |
| **Alert Banner** | Persistent red `Node` at top of HUD: `"⚠️ SANCTUARY NEGLECTED – PROGRESS HALTED!"` |
| **Animal Sad Emotes** | Spawn floating scribble-like emote sprites above animal entities. |
| **Economy Halt** | Coin generation = 0/hour. |

**Recovery:** Any single care action that raises the average above 0.30 restores normal visuals, music, and economy.

---

## 16. Education System

### Fact Cards

Each animal has an array of `FactCard` objects (defined in `animals.json`). Accessing them:

1. Player walks adjacent to an animal NPC.
2. Presses `Space` (the animal must be directly interactable, not through a tank/enclosure object).
3. A modal overlay displays the fact card with title, body text, and source.

### Upkeep Trivia

On completing the daily checklist (all stats at 100%), the HQ Clipboard displays a multiple-choice trivia question about active ambassadors:

- **Reward:** 15 coins for a correct answer.
- **No Penalty:** Incorrect answers give 0 coins but can be retried.

---

## 17. Data File Index

| File | Schema | Description |
|---|---|---|
| [`schemas/room.schema.json`](schemas/room.schema.json) | — | Room layout JSON Schema |
| [`schemas/animal.schema.json`](schemas/animal.schema.json) | — | Animal ambassador JSON Schema |
| [`schemas/item.schema.json`](schemas/item.schema.json) | — | Inventory item JSON Schema |
| [`schemas/economy.schema.json`](schemas/economy.schema.json) | — | Economy configuration JSON Schema |
| [`schemas/overview_map.schema.json`](schemas/overview_map.schema.json) | — | Overview map JSON Schema |
| [`schemas/event.schema.json`](schemas/event.schema.json) | — | Daily event JSON Schema |
| [`rooms/nutrition_house.json`](rooms/nutrition_house.json) | `room.schema.json` | Nutrition House room data |
| [`rooms/studio.json`](rooms/studio.json) | `room.schema.json` | The Studio room data |
| [`rooms/pasture.json`](rooms/pasture.json) | `room.schema.json` | The Pasture room data |
| [`rooms/hq_office.json`](rooms/hq_office.json) | `room.schema.json` | HQ Office room data |
| [`data/animals.json`](data/animals.json) | `animal.schema.json` | All animal ambassadors |
| [`data/items.json`](data/items.json) | `item.schema.json` | All inventory items |
| [`data/economy.json`](data/economy.json) | `economy.schema.json` | Economy, caretakers, shops |
| [`data/events.json`](data/events.json) | `event.schema.json` | Daily event pool |
| [`data/overview_map.json`](data/overview_map.json) | `overview_map.schema.json` | Overview map buildings |
