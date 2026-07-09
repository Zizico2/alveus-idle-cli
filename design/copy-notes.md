# Items, stamps, caretakers & events (copy mine)

Prose and shop flavor mined from the old design JSON. **Costs, weights, and capacities** that are not yet in Rust live as Planned ballparks in [`alveus-configs/README.md`](../crates/alveus-configs/README.md). Shipped item display names: `item_data` in that crate.

---

## Items

| Id | Display name | Category | Description |
|----|--------------|----------|-------------|
| `raw_veggie_tub` | Lettuce & Veggie Tub | raw_diet | Fresh lettuce, carrots, and leafy greens. Chop at the Prep Table before serving. |
| `prepared_veggie_diet` | Prepared Veggie Diet | prepared_diet | Chopped vegetables ready for Stompy's trough. |
| `tortoise_leafy_greens` | Tortoise Leafy Greens | prepared_diet | Leafy greens for Push Pop. **Shipped.** |
| `chicken_grains` | Chicken Grains | prepared_diet | Seeds and grains for Polly. **Shipped.** |
| `cricket_box` | Cricket Box | prepared_diet | Live crickets for Georgie. |
| `mini_mirror` | Mini Mirror | enrichment_toy | Shiny mirror Polly loves to peck. |
| `shiny_object` | Shiny Object | enrichment_toy | Reflective trinket for Stompy. |
| *(rework)* parrot diet raw/prepared | TBD | diet | Replace old `carnivore_raw_prep` / `prepared_carnivore_diet` (thawed prey for snake Siren). |
| `shed_snake_skin` | Shed Snake Skin | collectible | For **future snakes** (Epic 12), not Siren. Display at HQ. |

Prep sketch: veggie tub → Prep Table tapping chore (~5 taps) → prepared veggie diet.

---

## Caretakers

| Id | Name | Unlock (ballpark) | Perk | Lore |
|----|------|-------------------|------|------|
| `maya` | Maya Higa | Free | **Founder's Focus** — faster feeding/cleaning in Pasture (old: 25% faster / 0.75× time) | Founder of Alveus Sanctuary. |
| `kayla` | Kayla | 150 coins | **Reptile Whisperer** — re-tune for frog + **parrot** Studio enrich (old: +50% enrich happiness; +15% walk in enclosures) | Lead Animal Care Coordinator. |
| `connor` | Connor | 250 coins | **Enclosure Master** — cleaning speed doubled (old: 0.5× clean time) | Sanctuary builders and tool specialists. |

---

## Stamps (shop copy)

| Stamp | Cost | Description |
|-------|------|-------------|
| Stompy's Great Escape | 100 | Stompy running past Maya in a stream background. |
| Leaf Blower Duel | 150 | Maya fighting dust clouds with a leaf blower. |
| Georgie's Crown | 200 | Georgie under a paper crown. |
| **Siren memorial stamp** | ~250 | **Replace** old “Python Scarf” (Kayla wearing snake Siren). Tasteful community-honoring art. |
| Mico's Rant | 300 | Mico the Macaw shouting into a stream mic (future roster). |

---

## HQ decor

| Decor | Cost | Description |
|-------|------|-------------|
| Emu Print Rug | 75 | Patterned rug on the HQ floor. |
| Plushie Shelf | 150 | Miniature plush ambassadors. |
| Ambient Soundbox | 200 | Audio quotes / bird squawks when stepped on. |
| Sanctuary Plaque | 500 | Golden plaque for maximum upkeep. |

---

## Daily events (narrative)

Weights: see configs README Planned. Shed event is **not** for parrot Siren.

| Event | Weight | Story |
|-------|--------|-------|
| Rainy Day | 0.20 | Rain on the sanctuary. Stompy's trough covered; spigot enrich disabled — towel Stompy in the shelter instead. |
| Escaped Crickets | 0.15 | Crickets escape the Bug Jar Rack; catch 3 on the Studio floor before feeding Georgie. |
| Volunteer Day | 0.20 | A volunteer helps with one random cleaning chore; small coin reward (old: +10). |
| Normal Day | 0.30 | No special event. |
| *(deferred)* Snake Shed Day | ~0.15 | Extra misting + shed-skin trophy — **Noodle/Patchy**, not Siren. |
| *(add)* Parrot-appropriate Studio event | TBD | Optional Epic 8 replacement for shed day. |
