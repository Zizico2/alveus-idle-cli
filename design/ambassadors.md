# Ambassador care & education copy

Historical intent mined from the old design JSON. **Numbers** (decay, restores, placements) live in [`alveus-configs`](../crates/alveus-configs). **Lore corrections** live in [`ROADMAP.md`](../ROADMAP.md).

**Siren:** treat as Blue-fronted Amazon memorial / legacy ambassador. Snake mist/shed/scent-trail copy is **superseded** — see [Deferred: snake shed](#deferred-snake-shed-epic-12) and Epic 12.

---

## Polly (Silkie Chicken) — Nutrition House

### Care loop (intent)

| Action | Player-facing idea |
|--------|-------------------|
| Feed | Scoop **Chicken Grains** from the Seed Chest → fill Polly's feed bowl. |
| Clean | Sweep Polly's nesting area in the playpen (no item). |
| Enrich | Place a **Mini Mirror** at Polly's Enrichment Zone; she pecks her reflection. |

### Enrichment minigame (sketch)

- Id: `mirror_cluckathon`
- Trigger object: `polly_enrichment_post`

### Fact cards (education copy)

**Polly the Silkie**  
Silkie chickens are known for their fluffy, silk-like plumage. Unlike most chickens, their feathers lack barbicels, giving them a fur-like appearance. They are one of the most docile and friendly chicken breeds.

**Did You Know?**  
Silkie chickens have black skin and bones, blue earlobes, and five toes on each foot (most chickens have four). They originated in China over 2,000 years ago.

---

## Push Pop (Sulcata Tortoise) — Push Pop Enclosure

### Care loop (intent)

| Action | Player-facing idea |
|--------|-------------------|
| Feed | Scoop **Tortoise Leafy Greens** from the Diet Fridge (Nutrition House) → place in Push Pop's feeding dish. |
| Clean | Rake/tidy sandy floor; runtime uses poop piles + wheelbarrow → compost (see shipped cleaning configs). |
| Enrich | Scatter hay for burrowing / exploring (not shipped yet). |

### Fact cards

**Push Pop the Sulcata Tortoise**  
Sulcata tortoises (*Centrochelys sulcata*) are the third-largest tortoise species in the world. Native to the Sahara and Sahel, they can live over 100 years in captivity.

---

## Stompy (Emu) — Pasture

### Care loop (intent)

| Action | Player-facing idea |
|--------|-------------------|
| Feed | Chop **Lettuce & Veggie Tub** at Prep Table → dump **Prepared Veggie Diet** into the Wooden Trough. |
| Clean | Sweep dynamically spawned manure piles (old design: ~2–4 piles; each restore ~34% cleanliness). |
| Enrich | Place a **Shiny Object** on the Shiny Mirror Post; Stompy investigates. |

### Enrichment minigame (sketch)

- Id: `laser_chase`
- Trigger object: `shiny_mirror_post`

### Fact cards

**Stompy the Emu**  
Emus (*Dromaius novaehollandiae*) are native to Australia. They are the second-largest living bird by height. Fun fact: emus cannot walk backwards!

**Emu Diet**  
In the wild, emus eat a variety of plants, insects, and small animals. At Alveus, Stompy enjoys a prepared diet of fresh vegetables and leafy greens.

---

## Georgie (African Bullfrog) — Studio

### Care loop (intent)

| Action | Player-facing idea |
|--------|-------------------|
| Feed | Take a **Cricket Box** from the Bug Jar Rack → feed into Georgie's Glass Tank. |
| Clean | Clean tank water at the tank water access point. |
| Enrich | Play nature sounds on the Livestream Soundboard (old design: shared happiness with Studio roommate). |

### Enrichment minigame (sketch)

- Id: `bug_snapper`
- Trigger object: `georgie_tank`

### Fact cards

**Georgie the African Bullfrog**  
African bullfrogs (*Pyxicephalus adspersus*) are one of the largest frog species in the world. Males can grow up to 9 inches long and weigh over 4 pounds!

---

## Siren (Blue-fronted Amazon) — Studio (memorial / legacy)

Implement as **parrot** care and education (foraging, vocal enrichment, perch/aviary care, pet-trade themes). Do **not** ship snake mist, shed day, or scent-trail pathfinder.

### Care loop (intent — rewrite from old snake design)

| Action | Player-facing idea (starting point) |
|--------|-------------------------------------|
| Feed | Parrot-appropriate diet from Nutrition / Studio prep (replace thawed-prey fridge flow). |
| Clean | Aviary / perch / dish care — not glass misting for humidity shed. |
| Enrich | Foraging toy, vocal enrichment, or soundboard share with Georgie — not scent-trail misting. |

### Fact / memorial hooks (replace old python cards)

Use Epic 9 education + Epic 7 memorial framing. Themes to write toward:

- Blue-fronted Amazon (*Amazona aestiva*) natural history and vocal intelligence.
- Pet-trade / companion-parrot welfare education (compassionate, not grim).
- Tasteful memorial / legacy ambassador note (plaque or album copy — not morbid gameplay).

Old ball-python fact cards and `scent_explorer` minigame are **not** current design.

---

## Deferred: snake shed (Epic 12 — Noodle / Patchy)

Preserved from old Siren snake design so it is not lost:

| Element | Old intent |
|---------|------------|
| Shed day event | Milky eyes / dulled skin; **3** mist interactions instead of 1 |
| Cleaning | Mist enclosure glass for humidity |
| Enrichment | Mist climbing branches; scent-trail pathfinder minigame (`scent_explorer`) |
| Collectible | **Shed Snake Skin** → display at HQ |
| Fact: Shedding | Snakes shed as they grow; milky eyes and dull skin; extra humidity helps |

Assign these to future snake ambassadors, not Siren.
