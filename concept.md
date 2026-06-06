# Alveus Sanctuary Keeping: Creative Concept Document

Welcome to **Alveus Sanctuary Keeping**, a cozy, low-stress daily management game where players take on the role of Alveus Sanctuary staff members to care for real-world animal ambassadors. Designed to be played in **10-minute daily check-ins**, the game rewards consistency and care, providing a relaxing virtual wildlife sanctuary experience.

---

## 🌻 Core Pillars & Design Philosophy

1. **A Cozy Ritual:** The gameplay loop is structured around brief daily check-ins. Rather than encouraging hours of endless grinding, players are incentivized to visit their animals once a day to clean, feed, and enrich them, maintaining a steady flow of sanctuary donations.
2. **Authentic Alveus Lore:** Every animal, staff member, chore, and collectible is directly inspired by the actual Alveus Sanctuary, founded by Maya Higa. From Stompy's curiosity to the specific ingredients in the Nutrition House, the game is a love letter to the Alveus community.
3. **Compassionate Care:** Animals never die, get sick, or face harm. Neglect simply leads to them becoming dirty, hungry, or bored, which halts progress and prompts negative visual feedback (desaturation, sad animations, alert banners) until the player returns to care for them.
4. **Zero Dark Patterns (Non-Profit Ethics):** The game is a first-party Alveus marketing and educational tool. It rejects addictive optimization, predatory timers, gacha loops, energy caps, and intrusive alerts. Instead, it respects the player's time and focuses on direct support and animal education.

---

## 👨‍🌾 A Day in the Life of a Caretaker: The Step-by-Step Gameplay Loop

As a 2D top-down grid-based game (analogous to classic *Pokémon* games), all player interactions are driven by grid navigation, directional movement (W/A/S/D or Arrow keys), and a single interaction key (`Space` or `E`). 

Here is the exact sequence a player experiences during their daily check-in:

### Step 1: Starting the Shift (HQ Log-In)
1. **Spawn:** The player spawns in the **Alveus HQ Office** next to the main desk.
2. **Offline Summary Pop-up:** A daily check-in report slides onto the screen:
   > `"Good morning! While you were away: Alveus accumulated 320 Coins. Sanctuary Upkeep decayed to 58%."`
3. **Checking the Clipboard:** The player walks over to the **Sanctuary Clipboard** hanging on the wall at `(x: 8, y: 7)` and presses `Space`. A clean retro UI window displays the status of all enclosures:
   * **Nutrition House (Polly):** Cleanliness: *95%*, Hunger: *42%* (Hungry), Happiness: *50%* (Bored).
   * **The Studio (Georgie & Siren):** Cleanliness: *30%* (Dirty Tank), Hunger: *80%*, Happiness: *20%* (Bored).
   * **The Pasture (Stompy):** Cleanliness: *10%* (Manure Piles), Hunger: *15%* (Starving), Happiness: *85%*.

### Step 2: Preparing the Diets (The Nutrition House)
The player needs to gather and prepare food before they can feed any animals.
1. **Travel:** Walk out of the HQ building, navigate down the sandy pathway on the overview map, and approach the door of the **Nutrition House**.
2. **Enter:** Step onto the building's entrance tiles. A toast notification slides in from the bottom: `[Press Enter to enter Nutrition House]`. Pressing `Enter` loads the interior room.
3. **The Fridge Menu:** The player walks over to the industrial stainless-steel **Reptile/Mammal Fridge** at `(x: 2, y: 8)` and presses `Space`. A grid menu overlays on screen:
   * Select `[1] Lettuce & Veggie Tub` (for Stompy)
   * Select `[2] Cricket Box` (for Georgie)
   * Select `[3] Carnivore Raw Prep` (for Mia the Hawk)
   * *The player selects Option 1 (Lettuce & Veggie Tub). A floating icon of a salad bowl appears above the player's character, indicating they are carrying a raw diet.*
4. **The Prep Counter (Chop Mini-Chore):** The player walks to the **Prep Table** at `(x: 5, y: 7)` and presses `Space`. A quick micro-game triggers: a chopping board icon appears, and the player must tap `Space` 5 times in rapid succession to chop the veggies. Once finished, the diet becomes a "Prepared Veggie Diet" and is stored in their **Caretaker Satchel** (inventory can hold up to 2 items).
5. **The Seed Chest (Chicken Feed):** The player walks to the wooden **Seed Chest** at `(x: 2, y: 4)` and presses `Space` to scoop a bag of chicken grains for Polly. This fills the second inventory slot.

### Step 3: Feeding & Chores inside the Nutrition House (Caring for Polly)
Before leaving the Nutrition House, the player cares for Polly, who resides in a playpen on the right side of the room.
1. **Feeding:** Walk to Polly’s **Feed Bowl** at `(x: 8, y: 3)`. Press `Space` to empty the Chicken Grains. Polly (a fluffy white Silkie chicken sprite) wanders over and plays a pecking animation. Her **Hunger** stat bar on the HUD fills to 100%.
2. **Enrichment:** Walk to Polly's **Enrichment Zone** at `(x: 8, y: 5)`. Press `Space` to place a shiny mini-mirror. Polly stands in front of it, pecking happily at her reflection. Her **Happiness** bar fills to 100%.
3. **Exit:** The player walks to the exit door at `(x: 5, y: 0)` to return to the sanctuary overview map.

### Step 4: Outpost Travel & Pasture Maintenance (Caring for Stompy)
1. **Travel:** Walk east across the grass path to the **Emu Pasture Gates**. Enter the Pasture room.
2. **Feeding:** Walk up to the wooden **Pasture Trough** at `(x: 8, y: 3)` and press `Space` to dump the "Prepared Veggie Diet". Stompy (a large grey emu sprite) runs over and bobs his head to eat. His **Hunger** bar fills to 100%.
3. **Cleaning the Manure:** The pasture has accumulated 3 large brown **Manure Piles** spawned dynamically at random grid locations (e.g., `(x: 4, y: 6)`, `(x: 10, y: 4)`). The player walks directly adjacent to a pile and presses `Space` to sweep. A sweeping particle effect plays, the pile disappears, and the pasture's **Cleanliness** bar increments. The player cleans all 3 piles to restore cleanliness to 100%.
4. **Exit:** Walk back to the pasture gates and exit.

### Step 5: Collecting Daily Rewards & Spending Coins
1. **End-of-Shift Check:** The player walks back to the **HQ Office** and interacts with the **Clipboard**. Because all animal stats are now at 100%, the clipboard shows:
   > `✨ Shift Completed! All animals fed, clean, and happy. Daily Bonus: +50 Coins collected.`
2. **Album Stamp Shopping:** The player walks to the **Stamp Desk** at `(x: 4, y: 8)` and presses `Space`. A catalog of historical stamps appears. They spend 150 Coins to purchase the *"Leaf Blower Duel"* stamp. It is pasted into their Stamp Album, where they can inspect it and read its funny lore description.
3. **Log Out:** The player closes the game, knowing their sanctuary will generate max passive coins over the next few hours while they go about their day!

---

## 🌪️ Sanctuary Operations Variety: Dynamic Daily Events

While the care routine remains constant (much like a real sanctuary), the game introduces **Daily Events** to add operational variety and keep the 10-minute check-ins fresh:

1. **Weather Shifts (e.g., The Rainy Day):**
   * *Visual:* The overview map and pasture feature falling rain and muddy patches.
   * *Gameplay Impact:* Stompy’s pasture trough is covered. The sprinkler enrichment is disabled. Instead, the player must towel Stompy off inside his shelter (direct interaction) to satisfy his enrichment requirement.
2. **Reptile Shed Day (e.g., Siren's Shed):**
   * *Visual:* Siren has a milky eye overlay and a slightly dulled skin tone.
   * *Gameplay Impact:* Siren’s enclosure requires extra misting (3 misting interactions instead of 1) to help her shed. Cleaning her tank yields "Shed Snake Skin", which can be brought to the HQ and mounted on the wall as a display trophy.
3. **Escaped Feeder Crickets (e.g., Studio Insect Alert):**
   * *Gameplay Impact:* Instead of scooping crickets from the Bug Jar, the crickets have escaped! The player must walk around the Studio floor grid and press `Space` to catch 3 jumping crickets before they can feed Georgie.
4. **Volunteer Day:**
   * *Gameplay Impact:* Friendly local volunteer NPCs spawn in enclosures. They take over one random cleaning chore for the player (e.g., shoveling one manure pile in the pasture), acting as a friendly helper event.

---

## 🧸 Enrichment Mechanics: Active Chores & Future Minigames

Unlike food preparation, which always follows a "fetch-and-prepare" logic, enrichment is designed to be more varied. It is split into **Active Enrichment** (interacting directly with structures) and **Item-Based Enrichment** (fetching and placing toys). 

### How Enrichment Works Now
1. **Active Environment Control:** The player interacts directly with the room's fixtures.
   * *Example (The Studio):* The player walks to the **Livestream Soundboard** at `(7, 7)` and presses `Space`. An audio waves graphic plays on the HUD and nature sounds play. Georgie and Siren immediately receive a 50% boost to Happiness.
   * *Example (The Pasture):* The player interacts with the **Water Spigot** at `(1, 10)` to turn on pasture sprinklers, giving Stompy a refreshing mist.
2. **Item-Based Toy Placement:** The player retrieves a toy from the **Enrichment Locker** in the HQ or buys a premium toy (like a *Seed Puzzle Board*) from the Stamp Desk. They carry it in their satchel, walk to the enclosure's **Enrichment Zone**, and press `Space` to deploy it. The animal wanders over to play, filling their Happiness bar.

---

### 🕹️ Future Vision: Core Enrichment Minigames
To make daily check-ins more engaging, each ambassador animal will have a unique, playable puzzle or rhythm minigame that serves as their enrichment chore.

#### 1. Polly's Mirror Cluck-a-Thon (Rhythm Matching)
* **The Setup:** Triggers when placing the mini-mirror in Polly's pen.
* **The Minigame:** A simple quick-time event. Polly dances in front of the mirror, and arrow cues (`Up`, `Down`, `Left`, `Right`) glide across the screen. 
* **Mechanic:** The player must press the matching movement key at the right time. Matching 5 cues in a row makes Polly do a happy spin, completing the chore and granting a 10-coin bonus.

#### 2. Stompy's Shiny Laser Chase (Grid Steering)
* **The Setup:** Triggers when interacting with the Shiny Mirror Post in the Pasture.
* **The Minigame:** The player controls a circular red dot (laser pointer/reflector beam) on the pasture grid. Stompy will always walk toward the tile containing the red dot.
* **Mechanic:** The player must guide Stompy along a winding path of marked target tiles in the pasture. Moving the dot too fast breaks Stompy's attention, requiring you to lure him back. Guiding Stompy to step on 3 checkpoints completes the game.

#### 3. Siren's Scent Explorer (Snake Pathfinder)
* **The Setup:** Triggers when misting Siren's climbing branches.
* **The Minigame:** An overlay shows Siren's empty tank grid. 
* **Mechanic:** The player draws a scent trail (using quail scent or mouse scent) through the grid tiles, routing past branches and hiding spots. Once drawn, Siren slithers along the path. The more bends and climbing tiles Siren crosses, the higher her final enrichment value.

#### 4. Georgie's Timed Bug Snapper (Target Aiming)
* **The Setup:** Triggers when feeding crickets in Georgie's tank.
* **The Minigame:** Insects (crickets and beetles) hop across the screen in Georgie's tank grid at different speeds.
* **Mechanic:** A target box moves back and forth. The player must press `Space` at the exact moment a bug overlaps with the target box. Georgie shoots out his long tongue to snap it up. Snapping 3 bugs completes the game.

---

## 🌿 Educational Integration: Fact Cards & Trivia

To fulfill the educational mission of a real conservation non-profit, caretaking chores trigger educational interactions:

* **Ambassador Fact Cards:** When the player cleans, feeds, or enriches an animal, they can interact directly with the animal to view their **Fact Card** (curated by Alveus educators).
  * *Example Stompy Card:* `"Emu (Dromaius novaehollandiae) are native to Australia. They are the second-largest living bird by height. Fun Fact: Emus cannot walk backwards!"`
* **Upkeep Trivia:** Completing your daily checklist unlocks a quick multiple-choice question on the HQ desk clipboard based on the active ambassadors. Answering correctly awards a bonus of **15 Coins**, encouraging players to actually read and learn about the animals.

---

## 💖 The Alveus Supporter Program: Ethical Microtransactions

Monetization is designed strictly around the concept of supporting a physical wildlife sanctuary. The game behaves like an extension of Alveus's Twitch subscriptions or Patreon tiers, completely avoiding manipulative sales tactics.

### 🌟 Core Ethics Rules
* **No Pay-to-Win/Fast-Forward:** Players cannot buy coins, speed up stat regeneration, or pay to skip chores.
* **Highly Explicit Placement:** Transactions are confined to a single, static **Support Alveus** menu on the Title Screen and the HQ Desk. There are no pop-ups, limited-time warnings, or flashing sale buttons.
* **Completely Optional:** Purchases are strictly cosmetic or collectible, having zero impact on core gameplay loop progression.

### Supporter Purchase Types
1. **The Monthly Commemorative Stamp ($1.00 / Month):**
   * Akin to a Twitch Sub, a new special stamp is released every month depicting a recent real-world Alveus milestone (e.g., *"Stompy's Birthday bash"*).
   * Purchasing it places it in a dedicated "Supporter" page in the Stamp Album and unlocks a matching flag/banner that can be placed outside the HQ building.
2. **Caretaker Outfit Packs ($2.00 / One-Off):**
   * Optional cosmetic skins for the playable staff.
   * *Skins:* "Safari Guide Maya", "Frog Onesie Kayla", "Hardhat Connor".
3. **Direct Sanctuary Donation Button:**
   * A clear link on the HQ desk that exits the game and takes the player directly to the official [Alveus Sanctuary donation page](https://www.alveussanctuary.org/donate).

---

## 🗺️ Detailed Interior Room Layouts & Grid Objects

To implement these mechanics in Bevy's 2D grid engine, each room has a strict coordinate map and layout scheme:

```
NUTRITION HOUSE (11x11 Grid)
[W][W][W][W][W][D][W][W][W][W][W]  <- Row 10 (Walls, Exit Door at x=5, y=0 in Bevy coordinates)
[W]  [F]         [C]         [W]  <- Fridge [F] at x=2, y=8 | Smoothie Blender [C] at x=7, y=8
[W]              [P]         [W]  <- Prep Table [P] at x=5, y=7
[W]  [S]                     [W]  <- Seed Chest [S] at x=2, y=5
[W]                     [E]  [W]  <- Polly's Enrichment Post [E] at x=8, y=4
[W]                     [B]  [W]  <- Polly's Feed Bowl [B] at x=8, y=3
[W]                     [P]  [W]  <- Polly's Nesting Box [P] at x=8, y=2
[W]                          [W]
[W][W][W][W][W][W][W][W][W][W][W]  <- Row 0 (Bottom Wall)
```

### 1. The Nutrition House Layout
* **Dimensions:** 11x11 tiles.
* **Key Coordinates:**
  * `(x: 5, y: 0)`: Room exit door (leads back to overview coordinate `x: 33, y: 12`).
  * `(x: 2, y: 8)`: **Diet Fridge** (Obstacle, Interactable) — Opens the fridge menu.
  * `(x: 5, y: 7)`: **Prep Table** (Obstacle, Interactable) — Triggers chopping mini-chore.
  * `(x: 7, y: 8)`: **Smoothie Blender** (Obstacle, Interactable) — Triggers smoothie-making rhythm mini-chore.
  * `(x: 2, y: 5)`: **Seed Chest** (Obstacle, Interactable) — Gives chicken grains.
  * `(x: 7..=9, y: 1..=5)`: **Polly's Playpen** (Enclosed by fence obstacles, entry gate at `x: 7, y: 3`).

### 2. The Studio Layout
* **Dimensions:** 15x15 tiles.
* **Key Coordinates:**
  * `(x: 7, y: 0)`: Room exit door (leads back to overview coordinate `x: 15, y: 14`).
  * `(x: 3, y: 12)`: **Georgie's Glass Tank** (Obstacle, Interactable) — Feed crickets / Clean tank water.
  * `(x: 11, y: 12)`: **Siren's Warm Enclosure** (Obstacle, Interactable) — Mist glass / Clean branch.
  * `(x: 7, y: 7)`: **Livestream Soundboard & Cameras** (Obstacle, Interactable) — Play music for enrichment.
  * `(x: 13, y: 2)`: **Bug Jar Rack** (Obstacle, Interactable) — Retrieve crickets.

### 3. The Pasture Layout
* **Dimensions:** 21x21 tiles.
* **Key Coordinates:**
  * `(x: 10, y: 0)`: Pasture gates (leads back to overview coordinate `x: 20, y: 24`).
  * `(x: 4, y: 17)`: **Stompy's Shelter** (Obstacle).
  * `(x: 15, y: 15)`: **Emu Feed Bin** (Obstacle, Interactable) — Retrieve veggie bin/lettuce.
  * `(x: 10, y: 10)`: **Wooden Trough** (Obstacle, Interactable) — Place emu food.
  * `(x: 18, y: 8)`: **Shiny Mirror Post** (Interactable) — Place shiny object for enrichment.
  * *Manure Piles* spawn dynamically on random free pasture grass tiles every day.

### 4. The HQ Office Layout
* **Dimensions:** 11x11 tiles.
* **Key Coordinates:**
  * `(x: 5, y: 0)`: Room exit door (leads back to overview coordinate `x: 25, y: 9`).
  * `(x: 2, y: 8)`: **Staff Roster Board** (Interactable) — Switch characters or hire Kayla/Connor/Brodie.
  * `(x: 8, y: 8)`: **Stamp Album Desk** (Interactable) — Open the Stamp Album shop and view collectibles.
  * `(x: 5, y: 7)`: **Main Sanctuary Desk** (Obstacle, Clipboard Interactable) — Inspect overall stats and collect daily complete bonus.

---

## 👩‍🌾 The Caretaking Team (Playable Characters)

Players start as Maya, but can unlock other staff members using Alveus Coins generated from sanctuary upkeep. Each caretaker has unique perks that affect gameplay and visuals.

| Caretaker | Circle Color / Visuals | Unlock Cost | Special Perk | Lore Connection |
| :--- | :--- | :--- | :--- | :--- |
| **Maya Higa** | Emerald Green | *Default* | **Founder's Focus:** 25% faster at feeding/cleaning Emu and raptor enclosures. | Founder of Alveus Sanctuary. |
| **Kayla** | Mint Teal | 150 Coins | **Reptile Whisperer:** Frog and snake enrichment actions generate 50% more happiness. Walks 15% faster inside enclosures. | Lead Animal Care Coordinator. |
| **Connor** | Amber Gold | 250 Coins | **Enclosure Master:** Sweeping, wiping, and cleaning speed is doubled. | Sanctuary builders and tool specialists. |

---

## ⏳ Stats, Decay, and the Upkeep Cycle

Animals have three core attributes that decay in real time over a 24-to-48-hour cycle:

1. **Hunger (0.0 – 1.0):** Decays by `0.04/hour` (~25 hours to go empty). Replenished by feeding chores.
2. **Cleanliness (0.0 – 1.0):** Decays by `0.03/hour` (~33 hours to go dirty). Replenished by cleaning chores.
3. **Happiness (0.0 – 1.0):** Decays by `0.05/hour` (~20 hours to go bored). Replenished by enrichment chores.

### Sanctuary Upkeep Score
The overall health of the sanctuary is the **average of all stats across all active animals**. This score directly drives the sanctuary's economy.

```
Upkeep Score = (Average Hunger + Average Cleanliness + Average Happiness) / 3
```

---

## 🪙 Passive Economy & "Neglect Freeze"

The gameplay loop is built on passive coin generation representing viewer/visitor donations:

* **Upkeep >= 80% (Excellent):** Sanctuary generates **20 Coins/Hour**.
* **Upkeep between 30% and 79% (Fair):** Sanctuary generates **10 Coins/Hour**.
* **Upkeep < 30% (Neglected):** Sanctuary generates **0 Coins/Hour**.

### ⚠️ The Neglect Freeze (Negative UI Reinforcement)
If the player does not check in for 2 days, stats will drop below the 30% threshold, trigger a "Neglect Freeze", and alter the game's atmosphere:
* **Visual Shift:** The vibrant palette desaturates into a somber, dusty gray-scale tone.
* **Sound Design:** The upbeat background music slows down into a soft, minor-key piano/acoustic track.
* **Alert Banner:** A persistent red banner flashes at the top of the HUD:
  > `⚠️ SANCTUARY NEGLECTED - PROGRESS HALTED! Feed and clean the animals to resume coin generation.`
* **Animal Behavior:** Animals sit in place with sad, scribble-like emotes floating above them.

*As soon as the player cleans one room or feeds one animal, the average climbs back up, restoring color, music, and progression.*

---

## 📬 Progression Sinks & Collectibles

To give players long-term goals once they have unlocked all staff members, Alveus Coins can be spent on various cosmetic and historical collectibles.

### 1. The HQ Stamp Album
Located in the **Alveus HQ Office**, players can interact with a desk to open their Stamp Album. Stamps represent iconic Alveus stream moments:

* **"Stompy's Great Escape" (100 Coins):** Depicts Stompy running past Maya in the background of a stream.
* **"Leaf Blower Duel" (150 Coins):** Depicts Maya fighting off dust clouds with a leaf blower.
* **"Georgie's Crown" (200 Coins):** Depicts Georgie sitting under a paper crown.
* **"Python Scarf" (250 Coins):** Depicts Kayla wearing Siren like a scarf.
* **"Mico's Rant" (300 Coins):** Depicts Mico the Macaw shouting directly into a stream microphone.

### 2. HQ Customization
Buy furniture, decor, and interactive items to customize the HQ room:
* **Emu Print Rug (75 Coins):** Spawns a patterned rug on the floor.
* **Plushie Shelf (150 Coins):** Displays miniature plush versions of all the ambassadors.
* **Ambient Soundbox (200 Coins):** Plays audio quotes or special bird squawks when stepped on.
* **Sanctuary Plaque (500 Coins):** A golden wall plaque celebrating maximum upkeep.
