#!/usr/bin/env python3
"""Playtest: Nutrition House prep → feed Polly → enrich (+ nesting clean).

Navigation uses explicit tile-count routes with a CurrentTilePosition read after
every Move (AGENTS.md §4). No pathfinding helpers.

Requires a realtime headless server, e.g.:
  cargo run --features headless -- --headless --realtime --port 15702 --no-stdio
"""

from __future__ import annotations

import json
import sys
import time
import urllib.error
import urllib.request

PORT = 15702
BASE = f"http://127.0.0.1:{PORT}/"
EVENT = "alveus_headless::command::GameCommand"

# Planning hints — always confirm via player_tile() after each step (AGENTS.md §4).
OVERVIEW_SPAWN = (0, 0)
NUTRITION_ENTRANCE = (33, 12)
# Overview tile counts from spawn (0,0): climb off y=0, then east to entrance.
NAV_UP = 12
NAV_RIGHT = 33

NUTRITION_ROOM_SPAWN = (5, 2)
# Approach tiles (orthogonally adjacent to stations / interactables).
FRIDGE_APPROACH = (2, 7)
PREP_APPROACH = (5, 6)
SEED_CHEST_APPROACH = (2, 4)
TOY_BIN_APPROACH = (3, 4)
FEED_APPROACH = (7, 3)  # floor adjacent to bowl (8,3)
NESTING_APPROACH = (8, 2)  # adjacent to nesting (9,2)
ENRICH_APPROACH = (7, 4)  # adjacent to enrichment (7,5)

POLLY_FEED_BOWL = (8, 3)
POLLY_NESTING = (9, 2)
POLLY_ENRICHMENT = (7, 5)

# Polly wander idle is 2s; wait through several cycles before giving up.
BLOCK_RETRY_TIMEOUT_S = 8.0
BLOCK_RETRY_POLL_S = 0.5


def rpc(method: str, params=None):
    payload = {"jsonrpc": "2.0", "method": method, "id": 1}
    if params is not None:
        payload["params"] = params
    req = urllib.request.Request(
        BASE,
        data=json.dumps(payload).encode(),
        headers={"Content-Type": "application/json"},
        method="POST",
    )
    with urllib.request.urlopen(req, timeout=30) as resp:
        body = json.load(resp)
    if "error" in body:
        raise RuntimeError(f"BRP error for {method}: {body['error']}")
    return body.get("result")


def trigger(value) -> None:
    rpc("world.trigger_event", {"event": EVENT, "value": value})


def wait_for_http(timeout_s: float = 120.0) -> None:
    deadline = time.time() + timeout_s
    while time.time() < deadline:
        try:
            rpc("rpc.discover", {})
            return
        except (urllib.error.URLError, TimeoutError, RuntimeError):
            time.sleep(0.25)
    raise TimeoutError("BRP HTTP server did not become ready")


def parse_tile_position(raw) -> tuple[int, int] | None:
    if raw is None:
        return None
    if isinstance(raw, dict):
        if "x" in raw and "y" in raw:
            return int(raw["x"]), int(raw["y"])
        if "0" in raw:
            return parse_tile_position(raw["0"])
    return None


def player_tile() -> tuple[int, int] | None:
    res = rpc("world.query", {
        "data": {
            "components": ["alveus_components::CurrentTilePosition"],
            "has": [],
        },
        "filter": {"with": ["alveus_components::Player"]},
    })
    row = (res or [None])[0]
    if not row:
        return None
    raw = row.get("components", {}).get(
        "alveus_components::CurrentTilePosition"
    )
    return parse_tile_position(raw)


def step(direction: str, hold_s: float = 0.35) -> tuple[int, int] | None:
    before = player_tile()
    trigger({"Move": direction})
    time.sleep(hold_s)
    trigger("MoveStop")
    time.sleep(0.05)
    after = player_tile()
    if after == before:
        print(f"  blocked at {after} after Move {direction}", flush=True)
    return after


def follow(directions: list[str], expect: tuple[int, int]) -> tuple[int, int] | None:
    """Execute an explicit direction sequence; require the final queried tile."""
    print(f"  follow {directions} → expect {expect} from {player_tile()}", flush=True)
    pos = player_tile()
    for direction in directions:
        before = pos
        pos = step(direction)
        if pos == before:
            # Polly may occupy the next tile; poll until she moves or timeout.
            deadline = time.time() + BLOCK_RETRY_TIMEOUT_S
            while pos == before and time.time() < deadline:
                print(
                    f"  blocked at {pos}; polly={polly_tile()}; "
                    f"retrying {direction}",
                    flush=True,
                )
                time.sleep(BLOCK_RETRY_POLL_S)
                pos = step(direction)
            if pos == before:
                print(f"  FAIL: blocked on {direction}, still at {pos}", flush=True)
                return pos
    if pos != expect:
        print(f"  FAIL: expected {expect}, at {pos}", flush=True)
    return pos


def is_adjacent(a: tuple[int, int], b: tuple[int, int]) -> bool:
    return abs(a[0] - b[0]) + abs(a[1] - b[1]) == 1


def satchel_slots() -> list:
    res = rpc("world.get_resources", {
        "resource": "alveus_interaction::PlayerSatchel",
    })
    if isinstance(res, dict):
        slots = res.get("value", {}).get("slots")
        if isinstance(slots, list):
            return slots
        if "slots" in res:
            return res["slots"]
    return []


def satchel_has(needle: str) -> bool:
    for slot in satchel_slots():
        if slot is not None and needle in str(slot):
            return True
    return False


def player_entrance() -> str | None:
    res = rpc("world.query", {
        "data": {
            "components": ["alveus_components::BuildingEntrance"],
            "has": [],
        },
        "filter": {"with": ["alveus_components::Player"]},
    })
    row = (res or [None])[0]
    if not row:
        return None
    return str(
        row.get("components", {}).get(
            "alveus_components::BuildingEntrance"
        )
    )


def animal_stats(animal: str = "Polly") -> dict | None:
    res = rpc("world.query", {
        "data": {
            "components": [
                "alveus_types::AnimalId",
                "alveus_stats::AnimalStats",
            ],
            "has": [],
        },
        "filter": {"with": ["alveus_types::AnimalId"]},
    })
    for row in res or []:
        comps = row.get("components", {})
        aid = comps.get("alveus_types::AnimalId")
        if str(aid) == animal:
            return comps.get("alveus_stats::AnimalStats", {})
    return None


def polly_tile() -> tuple[int, int] | None:
    res = rpc("world.query", {
        "data": {
            "components": [
                "alveus_types::AnimalId",
                "alveus_stats::AnimalTilePosition",
            ],
            "has": [],
        },
        "filter": {"with": ["alveus_types::AnimalId"]},
    })
    for row in res or []:
        comps = row.get("components", {})
        if str(comps.get("alveus_types::AnimalId")) == "Polly":
            return parse_tile_position(
                comps.get("alveus_stats::AnimalTilePosition")
            )
    return None


def playpen_cleanliness() -> int | None:
    res = rpc("world.query", {
        "data": {
            "components": [
                "alveus_types::EnclosureId",
                "alveus_stats::EnclosureStats",
            ],
            "has": [],
        },
        "filter": {"with": ["alveus_types::EnclosureId"]},
    })
    for row in res or []:
        comps = row.get("components", {})
        enc = comps.get("alveus_types::EnclosureId")
        if enc == "NutritionHousePlaypen" or (
            isinstance(enc, dict) and enc.get(":variant") == "NutritionHousePlaypen"
        ):
            stats = comps.get("alveus_stats::EnclosureStats", {})
            return int(stats.get("cleanliness", 0))
    return None


def menu_state() -> str | None:
    for path in ("alveus_app::Menu", "bevy_state::state::State<alveus_app::Menu>"):
        try:
            res = rpc("world.get_resources", {"resource": path})
        except RuntimeError:
            continue
        if isinstance(res, dict):
            return str(res.get("value", res))
        if res is not None:
            return str(res)
    return None


def report(label: str) -> None:
    stats = animal_stats("Polly") or {}
    print(
        f"[{label}] tile={player_tile()} satchel={satchel_slots()} "
        f"menu={menu_state()} polly_hunger={stats.get('hunger')} "
        f"polly_happy={stats.get('happiness')} "
        f"playpen_clean={playpen_cleanliness()}",
        flush=True,
    )


def walk_overview_to_nutrition() -> tuple[int, int] | None:
    """Explicit overview route: Up × NAV_UP, Right × NAV_RIGHT → (33, 12)."""
    print(f"  overview: Up×{NAV_UP} then Right×{NAV_RIGHT}", flush=True)
    pos = player_tile()
    for _ in range(NAV_UP):
        pos = step("Up")
    for _ in range(NAV_RIGHT):
        pos = step("Right")
    if pos != NUTRITION_ENTRANCE:
        print(f"  FAIL: expected entrance {NUTRITION_ENTRANCE}, at {pos}", flush=True)
    return pos


def main() -> int:
    results: list[str] = []
    print("=== Polly Nutrition House care playtest ===", flush=True)

    wait_for_http()
    results.append("BRP ready")

    trigger("SkipSplash")
    time.sleep(0.5)
    trigger("Play")
    time.sleep(3.0)

    spawn = player_tile()
    report("gameplay start")
    if spawn != OVERVIEW_SPAWN:
        results.append(f"WARN: spawn {spawn}, expected {OVERVIEW_SPAWN}")
    else:
        results.append(f"Spawn OK at {spawn}")

    before = animal_stats("Polly") or {}
    hunger_before = before.get("hunger")
    happy_before = before.get("happiness")
    results.append(f"Polly before: hunger={hunger_before} happiness={happy_before}")
    if hunger_before is not None and int(hunger_before) > 200:
        trigger({
            "WorsenStat": {
                "target": {"Animal": {"id": "Polly", "stat": "Hunger"}},
                "amount": 600,
            }
        })
        time.sleep(0.2)
    if happy_before is not None and int(happy_before) > 200:
        trigger({
            "WorsenStat": {
                "target": {"Animal": {"id": "Polly", "stat": "Happiness"}},
                "amount": 600,
            }
        })
        time.sleep(0.2)
    before = animal_stats("Polly") or {}
    hunger_before = before.get("hunger")
    happy_before = before.get("happiness")
    results.append(f"Polly after worsen: hunger={hunger_before} happiness={happy_before}")

    # --- Enter Nutrition House ---
    if walk_overview_to_nutrition() != NUTRITION_ENTRANCE:
        results.append(f"FAIL: could not reach Nutrition House entrance ({player_tile()})")
        return 1
    report("nutrition entrance")
    if "NutritionHouse" not in str(player_entrance()):
        results.append(f"FAIL: not on Nutrition House entrance ({player_entrance()})")
        return 1

    trigger("EnterBuilding")
    time.sleep(3.0)
    report("inside nutrition house")
    if player_tile() != NUTRITION_ROOM_SPAWN:
        results.append(
            f"WARN: room spawn {player_tile()}, expected {NUTRITION_ROOM_SPAWN}"
        )

    # --- Fridge menu: take RawVeggieTub (first option) ---
    # (5,2) → Left×2 → (3,2) → Up×5 → (3,7) → Left → (2,7)
    if follow(["Left", "Left", "Up", "Up", "Up", "Up", "Up", "Left"], FRIDGE_APPROACH) != FRIDGE_APPROACH:
        results.append(f"FAIL: not adjacent to fridge, at {player_tile()}")
        return 1
    report("adjacent to fridge")
    trigger("Interact")
    time.sleep(0.4)
    report("fridge menu open")
    trigger("Continue")
    time.sleep(0.4)
    report("after fridge take")
    if not satchel_has("RawVeggieTub"):
        results.append(f"FAIL: expected RawVeggieTub, satchel={satchel_slots()}")
        return 1
    results.append("Fridge → RawVeggieTub")

    # --- Prep chop ---
    # (2,7) → Right → (3,7) → Down → (3,6) → Right×2 → (5,6)
    if follow(["Right", "Down", "Right", "Right"], PREP_APPROACH) != PREP_APPROACH:
        results.append(f"FAIL: not adjacent to prep, at {player_tile()}")
        return 1
    report("adjacent to prep table")
    trigger("Interact")
    time.sleep(0.4)
    report("after prep")
    if not satchel_has("PreparedVeggieDiet"):
        results.append(f"FAIL: expected PreparedVeggieDiet, satchel={satchel_slots()}")
        return 1
    results.append("Prep → PreparedVeggieDiet")

    # Free a slot for grains (drop prepared diet — still proves it existed).
    trigger("DropItem")
    time.sleep(0.2)

    # --- Seed chest → feed Polly ---
    # (5,6) → Left×2 → (3,6) → Down×2 → (3,4) → Left → (2,4)
    if follow(["Left", "Left", "Down", "Down", "Left"], SEED_CHEST_APPROACH) != SEED_CHEST_APPROACH:
        results.append(f"FAIL: not adjacent to seed chest, at {player_tile()}")
        return 1
    trigger("Interact")
    time.sleep(0.4)
    if not satchel_has("ChickenGrains"):
        results.append(f"FAIL: expected ChickenGrains, satchel={satchel_slots()}")
        return 1
    results.append("Seed chest → ChickenGrains")

    # (2,4) → Right×3 → (5,4) → Down → (5,3) → Right×2 → (7,3)
    if follow(
        ["Right", "Right", "Right", "Down", "Right", "Right"],
        FEED_APPROACH,
    ) != FEED_APPROACH:
        results.append(f"FAIL: not adjacent to feed bowl, at {player_tile()}")
        return 1
    if not is_adjacent(player_tile() or (0, 0), POLLY_FEED_BOWL):
        results.append(f"FAIL: {player_tile()} not adjacent to bowl {POLLY_FEED_BOWL}")
        return 1
    report("adjacent to feed bowl")
    hunger_before = (animal_stats("Polly") or {}).get("hunger")
    trigger("Interact")
    time.sleep(0.5)
    report("after feed")
    after_feed = animal_stats("Polly") or {}
    hunger_after = after_feed.get("hunger")
    if hunger_after is None:
        results.append("FAIL: could not read Polly hunger")
        return 1
    if satchel_has("ChickenGrains"):
        results.append(
            f"FAIL: feed did not consume grains (hunger {hunger_before}->{hunger_after})"
        )
        return 1
    results.append(f"Fed Polly: hunger {hunger_before} -> {hunger_after}")

    # --- Nesting clean ---
    clean_before = playpen_cleanliness()
    if clean_before is None:
        results.append("FAIL: could not read playpen cleanliness before sweep")
        return 1
    if clean_before >= 900:
        trigger({
            "WorsenStat": {
                "target": {
                    "Enclosure": {
                        "id": "NutritionHousePlaypen",
                        "stat": "Cleanliness",
                    }
                },
                "amount": 600,
            }
        })
        time.sleep(0.2)
        clean_before = playpen_cleanliness()
        if clean_before is None:
            results.append("FAIL: could not read playpen cleanliness after worsen")
            return 1
        results.append(f"Playpen cleanliness after worsen: {clean_before}")

    # (7,3) → Down → (7,2) → Right → (8,2)
    if follow(["Down", "Right"], NESTING_APPROACH) != NESTING_APPROACH:
        results.append(f"FAIL: not adjacent to nesting, at {player_tile()}")
        return 1
    reached = player_tile()
    if reached is None or not is_adjacent(reached, POLLY_NESTING):
        results.append(
            f"FAIL: {reached} not adjacent to nesting {POLLY_NESTING}"
        )
        return 1
    trigger("Interact")
    time.sleep(0.4)
    report("after nesting sweep")
    clean_after = playpen_cleanliness()
    if clean_after is None:
        results.append("FAIL: could not read playpen cleanliness after sweep")
        return 1
    if clean_after <= clean_before:
        results.append(
            f"FAIL: cleanliness did not increase ({clean_before} -> {clean_after})"
        )
        return 1
    results.append(f"Swept nesting: cleanliness {clean_before} -> {clean_after}")

    # --- Toy bin → enrich ---
    # (8,2) → Left → (7,2) → Up → (7,3) → Left×4 → (3,3) → Up → (3,4)
    if follow(
        ["Left", "Up", "Left", "Left", "Left", "Left", "Up"],
        TOY_BIN_APPROACH,
    ) != TOY_BIN_APPROACH:
        results.append(f"FAIL: not adjacent to toy bin, at {player_tile()}")
        return 1
    trigger("Interact")
    time.sleep(0.4)
    if not satchel_has("MiniMirror"):
        results.append(f"FAIL: expected MiniMirror, satchel={satchel_slots()}")
        return 1
    results.append("Toy bin → MiniMirror")

    # (3,4) → Right×2 → (5,4) → Down → (5,3) → Right×2 → (7,3) → Up → (7,4)
    enrich_pos = follow(
        ["Right", "Right", "Down", "Right", "Right", "Up"],
        ENRICH_APPROACH,
    )
    if enrich_pos != ENRICH_APPROACH:
        # Polly may occupy (7,4); stand at (8,5) instead (also adjacent to post).
        if player_tile() == (7, 3):
            enrich_pos = follow(["Up", "Right"], (8, 5))
        elif player_tile() == (7, 4):
            enrich_pos = follow(["Right"], (8, 5))
    reached = player_tile()
    if reached is None or not is_adjacent(reached, POLLY_ENRICHMENT):
        results.append(
            f"FAIL: {reached} not adjacent to enrichment {POLLY_ENRICHMENT}"
        )
        return 1
    happy_before = (animal_stats("Polly") or {}).get("happiness")
    trigger("Interact")
    time.sleep(0.5)
    report("after enrich")
    after_enrich = animal_stats("Polly") or {}
    happy_after = after_enrich.get("happiness")
    if satchel_has("MiniMirror"):
        results.append("FAIL: MiniMirror should be consumed on enrich")
        return 1
    results.append(f"Enriched Polly: happiness {happy_before} -> {happy_after}")

    print("\n=== Summary ===", flush=True)
    for line in results:
        print(f"  • {line}", flush=True)
    return 0


if __name__ == "__main__":
    sys.exit(main())
