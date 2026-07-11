# alveus-configs

**Source of truth for gameplay numbers** in alveus-idle.

| Kind | Where | Binding? |
|------|--------|----------|
| **Shipped** | Rust in [`src/lib.rs`](src/lib.rs) | Yes — runtime reads these |
| **Planned defaults** | This README (below) | No — ballparks to start from when implementing; promote into Rust, then delete from Planned |

Do **not** add new magic numbers in feature crates. Extend this crate (Rust) or add a Planned row here first.

Historical prose (care loops, fact cards, room objects, shop copy): [`design/`](../../design/) — especially [`ambassadors.md`](../../design/ambassadors.md), [`rooms.md`](../../design/rooms.md), [`copy-notes.md`](../../design/copy-notes.md). What to build next / lore: [`ROADMAP.md`](../../ROADMAP.md).

**Lore:** Siren is a **Blue-fronted Amazon** (memorial / legacy ambassador). Snake shed mechanics belong with future snakes (Noodle / Patchy), not Siren — see ROADMAP lore table.

---

## Shipped (in Rust)

| Domain | Symbols |
|--------|---------|
| Stat scale | `Stat`, `STAT_SCALE`, `STAT_FULL` |
| Timing / feel | `TILE_SIZE`, `PLAYER_MOVE_DURATION_SECS`, `AUTOSAVE_INTERVAL_SECS`, `LOADING_TIMEOUT_SECS` |
| Neglect | `NEGLECT_UPKEEP_THRESHOLD` |
| Satchel | `SATCHEL_MAX_SLOTS` (= 2) |
| Care restores | `FeedStat` / `EnrichStat` / `CleanStat`; `CARE_FEED_RESTORE`, `CARE_ENRICH_RESTORE`, `CARE_CLEAN_RESTORE` |
| Prep chore | `PREP_RECIPES`, `prep_recipe_for` |
| Care menus | `care_menu_options` (`CareMenuId::Fridge`) |
| Overview spawn | `OVERVIEW_PLAYER_SPAWN` |
| Items (5) | `item_data` / `item_display_name` |
| Animals | `ANIMALS_DATA`, `animal_data`, `enclosure_for_animal` |
| Enclosures | `ENCLOSURES_DATA`, `enclosure_data` |
| Placements | `POLLY_PLACEMENT`, `PUSH_POP_PLACEMENT`, `animal_default_placement` |
| Rooms | `NUTRITION_HOUSE_ROOM`, `PUSH_POP_ENCLOSURE_ROOM` |
| Cleaning | `WHEELBARROW_CAPACITY`, sparse `POOP_CONFIGS` / `poop_config_for` → `Option`, cleaning math fns |
| Offline wander | `OFFLINE_WANDER_STEPS_PER_HOUR` |

**Promotion rule:** When an epic implements a Planned row, move it into Rust, wire call sites, remove it from Planned. Do not leave a second authoritative copy in `design/`.

**Stat scale:** `Stat` is the shared discrete unit for hunger, happiness, and enclosure cleanliness. `STAT_SCALE: Stat = Stat(1000)` defines a full bar, and `STAT_FULL` aliases that typed value for defaults.

**Action wrappers:** Feed, enrichment, and cleaning authored deltas use `FeedStat`, `EnrichStat`, and `CleanStat` so call sites preserve the action domain while still converting to the shared `Stat` unit at mutation time. The `CARE_*_RESTORE` names are retained, but their values are typed action wrappers around `STAT_FULL`.

**Rates:** Decay rates remain `f32` because they accumulate fractional stat units over time. Convert with `Stat::get()` only at rate, percentage, logging, or serialization boundaries.

**Tiled override:** Per-object `FeedAnimal.delta` / `EnrichAnimal.delta` / `CleanAnimal.delta` on tiles use the matching stat newtype class shape (e.g. `alveus_types::FeedStat` with member `0`) and may override the typical care restore constants.

**Poop configs** are sparse / opt-in (`POOP_CONFIGS` via `phf_codegen`): only enclosures that use pile-based cleaning appear in the map. `poop_config_for` returns `None` for nest-sweep rooms (Nutrition House / Polly). Add a map entry when Pasture manure (etc.) ships.

---

## Planned defaults (ballparks)

Non-binding starting points for playtest tuning. Values below were mined from the old design JSON (now deleted). Prefer **parrot** framing for Siren; shed rows are under **future snakes**.

### Economy

| Knob | Ballpark | Notes |
|------|----------|-------|
| Coin tier Excellent | ≥ 0.80 upkeep → **20**/hour | |
| Coin tier Fair | ≥ 0.30 upkeep → **10**/hour | |
| Coin tier Neglected | &lt; 0.30 → **0**/hour | Aligns with `NEGLECT_UPKEEP_THRESHOLD` |
| Daily HQ bonus | **50** coins | All stats at 100% when checking clipboard |
| Upkeep formula | `(avg_hunger + avg_cleanliness + avg_happiness) / 3` | Already used in stats |

### Care restores (remaining)

Normalized `1.0` → `STAT_SCALE` (`Stat(1000)`) units when implemented.

| Action | Ballpark restore | Notes |
|--------|------------------|-------|
| Stompy clean (partial) | **0.34** | Old design; re-tune in Pasture epic |
| Georgie enrich (soundboard share) | **0.5** | Shared happiness bump |

Shipped: feed/enrich typical = `STAT_FULL`; prep is a single Interact (`PREP_RECIPES`).

### Unimplemented items

Full descriptions: [`design/copy-notes.md`](../../design/copy-notes.md).

| Id | Display name | Category | Notes |
|----|--------------|----------|-------|
| `cricket_box` | Cricket Box | prepared_diet | Georgie |
| `carnivore_raw_prep` | *(rework)* | raw_diet | Was snake prey — redesign for **parrot** Siren diet |
| `prepared_carnivore_diet` | *(rework)* | prepared_diet | Same — parrot-appropriate |
| `shiny_object` | Shiny Object | enrichment_toy | Stompy |
| `shed_snake_skin` | Shed Snake Skin | collectible | **Future snakes**, not Siren |

Shipped: `TortoiseLeafyGreens`, `ChickenGrains`, `RawVeggieTub`, `PreparedVeggieDiet`, `MiniMirror`.

### Daily events (weights)

Narrative copy: [`design/copy-notes.md`](../../design/copy-notes.md).

| Event | Weight | Room focus | Notes |
|-------|--------|------------|-------|
| Rainy Day | 0.20 | Pasture | Disable spigot; towel enrich in shelter |
| Escaped Crickets | 0.15 | Studio | Catch-3 chore before Georgie feed |
| Volunteer Day | 0.20 | Multi | +10 coins; help one clean chore |
| Normal Day | 0.30 | — | No-op |
| ~~Siren Shed Day~~ | ~~0.15~~ | — | **Deferred** → snake ambassadors (Epic 12); do not ship for parrot Siren |

### Collectibles (stamps)

Shop flavor: [`design/copy-notes.md`](../../design/copy-notes.md).

| Stamp | Cost | Lore note |
|-------|------|-----------|
| Stompy's Great Escape | 100 | Stream gag |
| Leaf Blower Duel | 150 | Maya / dust |
| Georgie's Crown | 200 | |
| ~~Python Scarf~~ | ~~250~~ | **Replace** with Siren **memorial** stamp (Epic 10) — not a snake scarf joke |
| Mico's Rant | 300 | Future macaw |

### HQ decor costs

| Decor | Cost |
|-------|------|
| Emu Print Rug | 75 |
| Plushie Shelf | 150 |
| Ambient Soundbox | 200 |
| Sanctuary Plaque | 500 |

Descriptions: [`design/copy-notes.md`](../../design/copy-notes.md).

### Caretaker unlock costs (ballpark)

Perk/lore prose: [`design/copy-notes.md`](../../design/copy-notes.md).

| Caretaker | Unlock | Perk sketch |
|-----------|--------|-------------|
| Maya | 0 | Founder's Focus — faster emu/raptor chores |
| Kayla | 150 | Studio enrich bonus (re-tune for parrot + frog) |
| Connor | 250 | Cleaning speed |

### Future room tiles (ballpark)

Object lists: [`design/rooms.md`](../../design/rooms.md). Care/education: [`design/ambassadors.md`](../../design/ambassadors.md).

| Room | Grid | Spawn | Exit door | Exit → overview |
|------|------|-------|-----------|-----------------|
| Studio | 15×15 | (7, 2) | (7, 0) | (15, 14) — verify vs Tiled when building |
| Pasture | 21×21 | (10, 2) | (10, 0) | (20, 24) |
| HQ Office | 11×11 | (5, 2) | (5, 0) | (25, 9) |

Siren Studio care (intent): parrot feeding, cleaning, foraging/vocal enrichment — **not** mist/shed. Memorial framing per ROADMAP Epic 7.

### Snake shed (Epic 12 — Noodle / Patchy)

| Knob | Ballpark |
|------|----------|
| Extra mist interactions on shed day | **3** |
| Reward | `shed_snake_skin` collectible → HQ display |
| Event weight | ~0.15 (was old Siren shed) |
