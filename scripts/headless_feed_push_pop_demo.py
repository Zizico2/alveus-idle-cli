#!/usr/bin/env python3
"""Playtest: Nutrition House fridge menu → greens → Push Pop Enclosure → feed.

Note: Diet Fridge is an OpenMenu (CareItemPicker). First option is RawVeggieTub;
Move NavigateListMenu Down then Continue selects TortoiseLeafyGreens for Push Pop.

Navigation uses explicit tile-count routes with a CurrentTilePosition read after
every Move (AGENTS.md §4). No pathfinding helpers.
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
PUSH_POP_ENTRANCE = (39, 12)
NAV_UP = 12
NAV_RIGHT_NUTRITION = 33
# From Nutrition House exit spawn (33, 12) east to Push Pop entrance (39, 12).
NAV_RIGHT_TO_PUSH_POP = 6

NUTRITION_ROOM_SPAWN = (5, 2)
FRIDGE_APPROACH = (2, 7)
PUSH_POP_ROOM_SPAWN = (6, 2)
# Feeding dish at (8, 6); stand south at (8, 5).
DISH_APPROACH = (8, 5)
PUSH_POP_DISH = (8, 6)


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
    print(f"  follow {directions} → expect {expect} from {player_tile()}", flush=True)
    pos = player_tile()
    for direction in directions:
        before = pos
        pos = step(direction)
        if pos == before:
            time.sleep(0.4)
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
    return []


def satchel_item() -> str | None:
    for slot in satchel_slots():
        if slot is not None:
            return str(slot)
    return None


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


def animal_hunger(animal: str = "PushPop") -> int | None:
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
            stats = comps.get("alveus_stats::AnimalStats", {})
            return int(stats.get("hunger", -1))
    return None


def report(label: str) -> None:
    print(
        f"[{label}] tile={player_tile()} satchel={satchel_item()} "
        f"entrance={player_entrance()} push_pop_hunger={animal_hunger('PushPop')}",
        flush=True,
    )


def walk_overview_counts(up: int, right: int, expect: tuple[int, int]) -> tuple[int, int] | None:
    print(f"  overview: Up×{up} then Right×{right} → {expect}", flush=True)
    pos = player_tile()
    for _ in range(up):
        pos = step("Up")
    for _ in range(right):
        pos = step("Right")
    if pos != expect:
        print(f"  FAIL: expected {expect}, at {pos}", flush=True)
    return pos


def main() -> int:
    results: list[str] = []
    print("=== Feed Push Pop playtest ===", flush=True)

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

    hunger_before = animal_hunger("PushPop")
    results.append(f"Push Pop hunger before: {hunger_before}")
    if hunger_before is not None and hunger_before > 200:
        trigger({
            "WorsenStat": {
                "target": {"Animal": {"id": "PushPop", "stat": "Hunger"}},
                "amount": 600,
            }
        })
        time.sleep(0.2)
        hunger_before = animal_hunger("PushPop")
        results.append(f"Push Pop hunger after worsen: {hunger_before}")

    # --- Overview → Nutrition House ---
    if walk_overview_counts(NAV_UP, NAV_RIGHT_NUTRITION, NUTRITION_ENTRANCE) != NUTRITION_ENTRANCE:
        results.append(f"FAIL: could not reach Nutrition House ({player_tile()})")
        return 1
    report("nutrition entrance")
    if "NutritionHouse" not in str(player_entrance()):
        results.append(f"FAIL: not on Nutrition House entrance ({player_entrance()})")
        return 1

    trigger("EnterBuilding")
    time.sleep(3.0)
    report("inside nutrition house")

    # --- Fridge: TortoiseLeafyGreens (second option) ---
    # (5,2) → Left×2 → (3,2) → Up×5 → (3,7) → Left → (2,7)
    if follow(
        ["Left", "Left", "Up", "Up", "Up", "Up", "Up", "Left"],
        FRIDGE_APPROACH,
    ) != FRIDGE_APPROACH:
        results.append(f"FAIL: not adjacent to fridge, at {player_tile()}")
        return 1
    report("adjacent to diet fridge")
    trigger("Interact")
    time.sleep(0.4)
    trigger({"NavigateListMenu": "Down"})
    time.sleep(0.15)
    trigger("Continue")
    time.sleep(0.5)
    item = satchel_item()
    report("after fridge menu take")
    if item and "TortoiseLeafyGreens" in item:
        results.append(f"Picked up greens: {item}")
    else:
        results.append(f"FAIL: expected TortoiseLeafyGreens in satchel, got {item}")
        return 1

    # ExitRoom works from anywhere in the interior.
    trigger("ExitRoom")
    time.sleep(3.0)
    report("back on overview")
    if player_tile() != NUTRITION_ENTRANCE:
        results.append(
            f"WARN: exit spawn {player_tile()}, expected {NUTRITION_ENTRANCE}"
        )

    # --- Overview → Push Pop Enclosure ---
    if walk_overview_counts(0, NAV_RIGHT_TO_PUSH_POP, PUSH_POP_ENTRANCE) != PUSH_POP_ENTRANCE:
        results.append(f"FAIL: could not reach Push Pop ({player_tile()})")
        return 1
    report("push pop entrance")
    if "PushPop" not in str(player_entrance()):
        results.append(f"FAIL: not on Push Pop entrance ({player_entrance()})")
        return 1

    trigger("EnterBuilding")
    time.sleep(3.0)
    report("inside push pop enclosure")

    if satchel_item() is None:
        results.append("FAIL: lost food before feeding")
        return 1

    # (6,2) → Up×3 → (6,5) → Right×2 → (8,5)
    if follow(["Up", "Up", "Up", "Right", "Right"], DISH_APPROACH) != DISH_APPROACH:
        # Push Pop may block (8,5); try (7,6) or (8,7) or (9,6).
        for alt_dirs, alt_tile in (
            (["Up"], (8, 6)),  # can't stand on dish
            (["Left", "Up"], (7, 6)),
            (["Right", "Up"] if player_tile() == (8, 5) else ["Right"], (9, 6)),
        ):
            if player_tile() is not None and is_adjacent(player_tile(), PUSH_POP_DISH):
                break
            follow(alt_dirs, alt_tile)
        if player_tile() is None or not is_adjacent(player_tile(), PUSH_POP_DISH):
            results.append(f"FAIL: not adjacent to dish, at {player_tile()}")
            return 1
    report("adjacent to feeding dish")
    hunger_before = animal_hunger("PushPop")
    trigger("Interact")
    time.sleep(0.5)
    report("after feed")

    hunger_after = animal_hunger("PushPop")
    satchel_after = satchel_item()
    results.append(f"Push Pop hunger after: {hunger_after}")
    results.append(f"Satchel after feed: {satchel_after}")

    if hunger_after is None:
        results.append("FAIL: could not read Push Pop hunger")
        return 1
    if satchel_after is not None:
        results.append("FAIL: satchel should be empty after feeding")
        return 1
    if hunger_before is not None and hunger_after <= hunger_before:
        results.append(
            f"FAIL: hunger did not increase ({hunger_before} -> {hunger_after})"
        )
        return 1

    results.append(f"Feed OK: hunger {hunger_before} -> {hunger_after}")

    print("\n=== Summary ===", flush=True)
    for line in results:
        print(f"  • {line}", flush=True)
    return 0


if __name__ == "__main__":
    sys.exit(main())
