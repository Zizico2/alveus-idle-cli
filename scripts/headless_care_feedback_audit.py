#!/usr/bin/env python3
"""BRP audit for player-reachable care feedback.

This driver uses only GameCommand verbs for actions. It walks the real
Nutrition House -> Push Pop route, picks up greens, feeds Push Pop, and checks
the resulting toast and ECS state. Clean/enrich copy that has no Epic 1 map
station is covered by Rust unit and in-process BRP tests instead of bypassing
the player interaction path with internal events.

Navigation uses explicit tile-count routes with a CurrentTilePosition read after
every Move (AGENTS.md §4). No pathfinding helpers.

Run against a fresh realtime headless server, then stop that server afterward:

  cargo run --features headless -- --headless --realtime --port 15702 --no-stdio
  python3 scripts/headless_care_feedback_audit.py
"""

from __future__ import annotations

import json
import os
import sys
import time
import urllib.error
import urllib.request

PORT = int(os.environ.get("BRP_PORT", "15702"))
BASE = f"http://127.0.0.1:{PORT}/"
EVENT = "alveus_headless::command::GameCommand"
REQUIRE_SERVER = os.environ.get("REQUIRE_SERVER", "0") == "1"

OVERVIEW_SPAWN = (0, 0)
NUTRITION_ENTRANCE = (33, 12)
PUSH_POP_ENTRANCE = (39, 12)
NAV_UP = 12
NAV_RIGHT_NUTRITION = 33
NAV_RIGHT_TO_PUSH_POP = 6

FRIDGE_APPROACH = (2, 7)
DISH_APPROACH = (8, 5)
PUSH_POP_DISH = (8, 6)

TEXT_CANDIDATES = (
    "bevy_ui::widget::text::Text",
    "bevy_text::Text",
    "bevy::prelude::Text",
)


def rpc(method: str, params=None):
    body = {"jsonrpc": "2.0", "id": 1, "method": method}
    if params is not None:
        body["params"] = params
    req = urllib.request.Request(
        BASE,
        data=json.dumps(body).encode(),
        headers={"Content-Type": "application/json"},
    )
    out = json.load(urllib.request.urlopen(req, timeout=30))
    if "error" in out:
        raise RuntimeError(out["error"])
    return out.get("result")


def trigger_game(value) -> None:
    rpc("world.trigger_event", {"event": EVENT, "value": value})


def get_resource(type_path: str):
    result = rpc("world.get_resources", {"resource": type_path})
    if isinstance(result, dict):
        return result.get("value", result)
    return result


def wait_for_http(timeout_s: float = 20.0) -> bool:
    deadline = time.time() + timeout_s
    while time.time() < deadline:
        try:
            rpc("rpc.discover", {})
            return True
        except (urllib.error.URLError, TimeoutError, RuntimeError, json.JSONDecodeError):
            time.sleep(0.25)
    return False


def parse_tile_position(raw) -> tuple[int, int] | None:
    if isinstance(raw, dict):
        if "x" in raw and "y" in raw:
            return int(raw["x"]), int(raw["y"])
        if "0" in raw:
            return parse_tile_position(raw["0"])
    return None


def player_tile() -> tuple[int, int] | None:
    result = rpc(
        "world.query",
        {
            "data": {
                "components": ["alveus_components::CurrentTilePosition"],
                "has": [],
            },
            "filter": {"with": ["alveus_components::Player"]},
        },
    )
    row = (result or [None])[0]
    if not row:
        return None
    return parse_tile_position(
        row.get("components", {}).get("alveus_components::CurrentTilePosition")
    )


def step(direction: str, hold_s: float = 0.35) -> tuple[int, int] | None:
    before = player_tile()
    trigger_game({"Move": direction})
    time.sleep(hold_s)
    trigger_game("MoveStop")
    time.sleep(0.05)
    after = player_tile()
    if after == before:
        print(f"blocked at {after} after Move {direction}", flush=True)
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


def player_entrance() -> str:
    result = rpc(
        "world.query",
        {
            "data": {
                "components": ["alveus_components::BuildingEntrance"],
                "has": [],
            },
            "filter": {"with": ["alveus_components::Player"]},
        },
    )
    row = (result or [None])[0]
    if not row:
        return ""
    return str(
        row.get("components", {}).get("alveus_components::BuildingEntrance", "")
    )


def satchel_slots() -> list:
    value = get_resource("alveus_interaction::PlayerSatchel")
    if isinstance(value, dict) and isinstance(value.get("slots"), list):
        return value["slots"]
    return []


def last_pickup_text() -> str | None:
    value = get_resource("alveus_components::LastPickupMessage")
    if not isinstance(value, dict):
        return None
    text = value.get("text")
    if isinstance(text, str):
        return text
    if isinstance(text, dict) and "Some" in text:
        return text["Some"]
    return None


def animal_hunger(animal: str = "PushPop") -> int | None:
    result = rpc(
        "world.query",
        {
            "data": {
                "components": ["alveus_types::AnimalId", "alveus_stats::AnimalStats"],
                "has": [],
            },
            "filter": {"with": ["alveus_types::AnimalId"]},
        },
    )
    for row in result or []:
        components = row.get("components", {})
        if str(components.get("alveus_types::AnimalId")) == animal:
            return int(components.get("alveus_stats::AnimalStats", {}).get("hunger", -1))
    return None


def extract_text(component) -> str | None:
    if isinstance(component, str):
        return component
    if isinstance(component, dict) and isinstance(component.get("0"), str):
        return component["0"]
    return None


def ui_texts() -> list[str]:
    for type_path in TEXT_CANDIDATES:
        try:
            result = rpc(
                "world.query",
                {
                    "data": {"components": [type_path], "has": []},
                    "filter": {"with": []},
                },
            )
        except RuntimeError:
            continue
        found = []
        for row in result or []:
            text = extract_text(row.get("components", {}).get(type_path))
            if text:
                found.append(text)
        if found:
            return found
    return []


def run_player_feed_flow() -> dict:
    trigger_game("SkipSplash")
    time.sleep(0.3)
    trigger_game("Play")
    time.sleep(2.0)
    if player_tile() != OVERVIEW_SPAWN:
        raise RuntimeError(f"expected fresh overview spawn, got {player_tile()}")

    trigger_game(
        {
            "WorsenStat": {
                "target": {"Animal": {"id": "PushPop", "stat": "Hunger"}},
                "amount": 600,
            }
        }
    )
    time.sleep(0.2)
    hunger_before = animal_hunger()

    if walk_overview_counts(NAV_UP, NAV_RIGHT_NUTRITION, NUTRITION_ENTRANCE) != NUTRITION_ENTRANCE:
        raise RuntimeError("could not reach Nutrition House entrance")
    if "NutritionHouse" not in player_entrance():
        raise RuntimeError(f"wrong entrance: {player_entrance()}")
    trigger_game("EnterBuilding")
    time.sleep(2.0)

    if follow(
        ["Left", "Left", "Up", "Up", "Up", "Up", "Up", "Left"],
        FRIDGE_APPROACH,
    ) != FRIDGE_APPROACH:
        raise RuntimeError(f"could not reach diet fridge, at {player_tile()}")
    # Fridge is CareItemPicker: first option RawVeggieTub, second TortoiseLeafyGreens.
    trigger_game("Interact")
    time.sleep(0.3)
    trigger_game({"Move": "Down"})
    time.sleep(0.15)
    trigger_game("MoveStop")
    time.sleep(0.1)
    trigger_game("Continue")
    time.sleep(0.4)
    if "TortoiseLeafyGreens" not in str(satchel_slots()):
        raise RuntimeError(f"greens not acquired: {satchel_slots()!r}")

    trigger_game("ExitRoom")
    time.sleep(2.0)
    if walk_overview_counts(0, NAV_RIGHT_TO_PUSH_POP, PUSH_POP_ENTRANCE) != PUSH_POP_ENTRANCE:
        raise RuntimeError("could not reach Push Pop entrance")
    if "PushPop" not in player_entrance():
        raise RuntimeError(f"wrong entrance: {player_entrance()}")
    trigger_game("EnterBuilding")
    time.sleep(2.0)

    if follow(["Up", "Up", "Up", "Right", "Right"], DISH_APPROACH) != DISH_APPROACH:
        pos = player_tile()
        if pos is None or not is_adjacent(pos, PUSH_POP_DISH):
            # Local alternate if Push Pop blocks (8,5).
            follow(["Left", "Up"], (7, 6))
            pos = player_tile()
            if pos is None or not is_adjacent(pos, PUSH_POP_DISH):
                raise RuntimeError(f"could not reach Push Pop feeding dish, at {pos}")
    trigger_game("Interact")
    time.sleep(0.35)

    return {
        "hunger_before": hunger_before,
        "hunger_after": animal_hunger(),
        "slots": satchel_slots(),
        "pickup": last_pickup_text(),
        "texts": ui_texts(),
    }


def main() -> int:
    if not wait_for_http():
        message = f"headless BRP not reachable at {BASE}"
        if REQUIRE_SERVER:
            print(f"FAIL: {message}", file=sys.stderr)
            return 1
        print(f"skip: {message}", file=sys.stderr)
        return 0

    try:
        result = run_player_feed_flow()
    except Exception as exc:  # noqa: BLE001 - driver boundary
        print(f"FAIL: {exc}", file=sys.stderr)
        return 1

    before = result["hunger_before"]
    after = result["hunger_after"]
    care_toast = next((text for text in result["texts"] if "Fed Push Pop" in text), None)
    failures = []
    if before is None or after is None or after <= before:
        failures.append(f"hunger did not improve: {before} -> {after}")
    if "TortoiseLeafyGreens" in str(result["slots"]):
        failures.append(f"feeding did not consume greens: {result['slots']!r}")
    if care_toast is None:
        failures.append(f"Fed Push Pop toast not found: {result['texts']!r}")
    if result["pickup"] is not None:
        failures.append(f"care success leaked into LastPickupMessage: {result['pickup']!r}")

    if failures:
        print("FAIL:", file=sys.stderr)
        for failure in failures:
            print(f"  - {failure}", file=sys.stderr)
        return 1

    print(f"PASS: player-fed Push Pop ({before} -> {after}); toast={care_toast!r}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
