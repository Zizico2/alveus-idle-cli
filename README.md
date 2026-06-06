# Alveus Idle CLI

A 2D tile-based simulation/idle game built in Rust using the **Bevy** game engine. The game is inspired by and themed around **Alveus Sanctuary**, the nonprofit wildlife sanctuary and virtual education center founded by streamer Maya Higa.

---

## 🎮 Game Concept & Overview

**Alveus Idle CLI** invites players to explore a digital recreation of Alveus Sanctuary. Players control an animal ambassador caretaker (conceptually represented by a duck, using `ducky.png` in the assets) and navigate the sanctuary grounds, entering various operational buildings to perform caretaking tasks.

### Core Mechanics
* **Grid-Based Tile Movement:** The player navigates a 32x32 pixel grid. Movement is snapped to individual tiles with smooth transition logic (`src/demo/movement.rs`).
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

* `src/main.rs`: Application entry point, plugin configuration, and camera setup.
* `src/components.rs`: Core component declarations (`TilePosition`, `BuildingEntrance`, `Obstacle`, etc.).
* `src/screens/`: Manages screen states (`Splash`, `Title`, `Loading`, `Gameplay`, and `InRoom`).
* `src/demo/`:
  * `level.rs`: Handles overview map spawning and player instantiation.
  * `player.rs`: Player controller, movement input logging, and asset loading.
  * `movement.rs`: Manages tile grid snapping and collision checks.
  * `entrance.rs`: Snapping checks for entrances and transition trigger events.
  * `room.rs`: Defines interior rooms, specifically building out the floor, walls, and obstacles of the **Nutrition House**.
  * `toast.rs`: A tween-animated toast UI notifying players of room entry prompts.
* `assets/`:
  * `images/`: Sprite and UI textures (such as `ducky.png`).
  * `maps/overview/`: Tiled `.tmx` maps, tilesets, and exports.
  * `audio/`: Footsteps and sound effects.

---

## 🚀 Getting Started

### Prerequisites
Make sure you have Rust and Cargo installed. If not, get them at [rustup.rs](https://rustup.rs/).

### Running the Game (Native)
To run the game locally in development mode (which includes hot-reloading for assets and Bevy dev tools):
```bash
cargo run
```

### Building for Release
To compile a highly-optimized release build:
```bash
cargo build --release
```
