#!/usr/bin/env python3
"""Playtest: Nutrition House → pick up greens → Push Pop Enclosure → feed → verify stats."""

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
NUTRITION_FRIDGE = (2, 8)          # TortoiseLeafyGreens (Diet Fridge)
NUTRITION_EXIT_DOOR = (5, 0)
PUSH_POP_ENTRANCE = (39, 12)
PUSH_POP_DISH = (8, 6)             # FeedAnimal → PushPop, requires TortoiseLeafyGreens


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


def walk_to(target: tuple[int, int], max_steps: int = 120) -> tuple[int, int] | None:
    tx, ty = target
    pos = player_tile()
    print(f"  walk_to {target} from {pos}", flush=True)
    for _ in range(max_steps):
        pos = player_tile()
        if pos is None:
            time.sleep(0.1)
            continue
        x, y = pos
        if (x, y) == (tx, ty):
            return pos
        if x < tx:
            step("Right")
        elif x > tx:
            step("Left")
        elif y < ty:
            step("Up")
        elif y > ty:
            step("Down")
    return player_tile()


def walk_adjacent_to(
    object_tile: tuple[int, int], max_steps: int = 80
) -> tuple[int, int] | None:
    """Reach any tile orthogonally adjacent to object_tile (for Interact)."""
    ox, oy = object_tile
    candidates = [(ox + 1, oy), (ox - 1, oy), (ox, oy + 1), (ox, oy - 1)]
    candidates = [(x, y) for x, y in candidates if x >= 0 and y >= 0]
    pos = player_tile()
    print(f"  walk_adjacent_to {object_tile} from {pos}", flush=True)
    for _ in range(max_steps):
        pos = player_tile()
        if pos is None:
            time.sleep(0.1)
            continue
        px, py = pos
        for cx, cy in candidates:
            if abs(px - cx) + abs(py - cy) == 0:
                return pos
        # Greedy toward nearest candidate
        best = min(candidates, key=lambda c: abs(c[0] - px) + abs(c[1] - py))
        bx, by = best
        if bx > px:
            step("Right")
        elif bx < px:
            step("Left")
        elif by > py:
            step("Up")
        elif by < py:
            step("Down")
    return player_tile()


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

    # --- Overview → Nutrition House ---
    walk_to(NUTRITION_ENTRANCE)
    report("nutrition entrance")
    if "NutritionHouse" not in str(player_entrance()):
        results.append(f"FAIL: not on Nutrition House entrance ({player_entrance()})")
        return 1

    trigger("EnterBuilding")
    time.sleep(3.0)
    report("inside nutrition house")

    # --- Pick up tortoise leafy greens ---
    walk_adjacent_to(NUTRITION_FRIDGE)
    report("adjacent to diet fridge")
    trigger("Interact")
    time.sleep(0.5)
    item = satchel_item()
    report("after pickup")
    if item and "TortoiseLeafyGreens" in item:
        results.append(f"Picked up greens: {item}")
    else:
        results.append(f"FAIL: expected TortoiseLeafyGreens in satchel, got {item}")
        return 1

    # --- Exit Nutrition House ---
    walk_to(NUTRITION_EXIT_DOOR)
    trigger("ExitRoom")
    time.sleep(3.0)
    report("back on overview")

    # --- Overview → Push Pop Enclosure ---
    walk_to(PUSH_POP_ENTRANCE)
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

    # --- Feed Push Pop ---
    walk_adjacent_to(PUSH_POP_DISH)
    report("adjacent to feeding dish")
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
    if hunger_before is not None and hunger_after <= hunger_before:
        results.append(
            f"FAIL: hunger did not increase ({hunger_before} -> {hunger_after})"
        )
        return 1
    if satchel_after is not None:
        results.append("FAIL: satchel should be empty after feeding")
        return 1

    results.append(f"Feed OK: hunger {hunger_before} -> {hunger_after}")

    print("\n=== Summary ===", flush=True)
    for line in results:
        print(f"  • {line}", flush=True)
    return 0


if __name__ == "__main__":
    sys.exit(main())
