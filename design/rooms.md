# Room & overview sketches

Object lists and layout intent mined from the old room JSON. **Shipped tile numbers** for Nutrition House / Push Pop live in [`alveus-configs`](../crates/alveus-configs). Future-room ballparks are in that crate’s README Planned section. ASCII layouts may also appear in [`design.md`](design.md).

Verify against Tiled maps and runtime when implementing — these are inspiration, not contracts.

---

## Overview map (sketch)

Old design spawn `(25, 10)` differs from shipped `OVERVIEW_PLAYER_SPAWN` `(0, 0)` — runtime wins.

| Building | Sketch position | Entrance zone (tiles) |
|----------|-----------------|------------------------|
| Nutrition House | (31, 14) | (32,13)–(35,14) |
| Push Pop Enclosure | (37, 14) | (38,33)–(41,34) — **verify**; may be stale vs current Tiled |
| Studio | (12, 12) | (14,13)–(17,14) |
| Pasture | (16, 22) | (19,23)–(22,24) |
| HQ Office | (22, 6) | (24,8)–(27,9) |

---

## Nutrition House

Spawn / exit / door: see shipped `NUTRITION_HOUSE_ROOM`.

| Object | Display name | Sketch pos | Interaction (intent) |
|--------|--------------|------------|----------------------|
| `diet_fridge` | Diet Fridge | (2, 8) | open menu (diets) |
| `prep_table` | Prep Table | (5, 7) | mini-chore (chop veggies — ~5 taps) |
| `smoothie_blender` | Smoothie Blender | (7, 8) | mini-chore |
| `seed_chest` | Seed Chest | (2, 5) | give item (chicken grains) |
| `polly_feed_bowl` | Polly's Feed Bowl | (8, 3) | feed Polly |
| `polly_enrichment_post` | Polly's Enrichment Zone | (8, 5) | enrich Polly |
| `polly_nesting_box` | Polly's Nesting Box | (8, 2) | prop / clean focus |

Polly home / wander: shipped `POLLY_PLACEMENT`.

---

## Push Pop Enclosure

Spawn / exit / door: see shipped `PUSH_POP_ENCLOSURE_ROOM` (note: old JSON `exit_target` was `(39, 16)`; runtime uses configs).

| Object | Display name | Sketch pos | Interaction (intent) |
|--------|--------------|------------|----------------------|
| `push_pop_feeding_dish` | Push Pop's Feeding Dish | (8, 6) | feed Push Pop |
| `push_pop_shelter` | Push Pop's Shelter | (3, 9) | prop |

Poop piles: dynamic spawn in wander bounds; wheelbarrow dump on **overview** compost (not inside this room). Shipped poop math: `poop_config_for` / `WHEELBARROW_CAPACITY`.

Push Pop home / wander: shipped `PUSH_POP_PLACEMENT`.

---

## Studio (Georgie + Siren parrot)

Ballparks: see configs README Planned (grid 15×15, spawn `(7,2)`, door `(7,0)`).

| Object | Display name | Sketch pos | Interaction (intent) |
|--------|--------------|------------|----------------------|
| `georgie_tank` | Georgie's Glass Tank | (3, 12) | feed Georgie (crickets) |
| `georgie_tank_clean` | Georgie's Tank Water | (3, 10) | clean |
| `livestream_soundboard` | Livestream Soundboard & Cameras | (7, 7) | toggle fixture / enrich |
| `bug_jar_rack` | Bug Jar Rack | (13, 2) | give cricket box |
| `studio_desk` | Studio Desk | (3, 7) | prop |
| `siren_enclosure` | Siren's enclosure / aviary | (11, 12) | **rework** for parrot (was mist chore) |
| `siren_enrichment_branch` | Siren's enrichment | (11, 10) | **rework** for parrot (was scent mist) |

Wander sketches: Georgie home `(4,12)` bounds `(3,11)–(5,13)`; Siren home `(12,12)` bounds `(11,11)–(13,13)` — re-tune for parrot space when building.

---

## Pasture (Stompy)

Ballparks: configs README (grid 21×21, spawn `(10,2)`).

| Object | Display name | Sketch pos | Interaction (intent) |
|--------|--------------|------------|----------------------|
| `stompy_shelter` | Stompy's Shelter | (4, 17) | prop / rainy-day towel enrich |
| `emu_feed_bin` | Emu Feed Bin | (15, 15) | give / store diet |
| `wooden_trough` | Wooden Trough | (10, 10) | feed Stompy |
| `shiny_mirror_post` | Shiny Mirror Post | (18, 8) | enrich |
| `water_spigot` | Water Spigot | (1, 10) | toggle; disabled on rainy day |

Stompy home sketch `(10,14)`, wander roughly full interior. Manure piles: dynamic cleanables (~2–4), ~0.34 cleanliness each (Planned).

---

## HQ Office

Ballparks: configs README (grid 11×11, spawn `(5,2)`).

| Object | Display name | Sketch pos | Interaction (intent) |
|--------|--------------|------------|----------------------|
| `staff_roster_board` | Staff Roster Board | (2, 8) | open staff menu |
| `stamp_album_desk` | Stamp Album Desk | (8, 8) | open stamp shop |
| `sanctuary_clipboard` | Sanctuary Clipboard | (5, 7) | daily checklist / trivia / event blurb |
| `donation_link_sign` | Support Alveus Sign | (5, 9) | external link (ethical support) |
| `hq_bookshelf` | Bookshelf | (1, 5) | prop |
| `hq_couch` | Office Couch | (8, 3) | prop |
