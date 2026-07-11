#!/usr/bin/env python3
"""Shared stdlib BRP helpers for headless Python drivers.

Navigation uses explicit direction lists / tile counts with a
CurrentTilePosition read after every Move (AGENTS.md §4). No pathfinding.
"""

from __future__ import annotations

import json
import os
import time
import urllib.error
import urllib.request

PORT = int(os.environ.get("BRP_PORT", "15702"))
BASE = f"http://127.0.0.1:{PORT}/"
EVENT = "alveus_headless::command::GameCommand"

# Opt-in skip only. Validation wrappers must never set this.
ALLOW_MISSING_SERVER = os.environ.get("ALLOW_MISSING_SERVER", "0") == "1"

OVERVIEW_SPAWN = (0, 0)
NUTRITION_ENTRANCE = (33, 12)
PUSH_POP_ENTRANCE = (39, 12)
NAV_UP = 12
NAV_RIGHT_NUTRITION = 33
NAV_RIGHT_TO_PUSH_POP = 6

NUTRITION_ROOM_SPAWN = (5, 2)
FRIDGE_APPROACH = (2, 7)
SEED_CHEST_APPROACH = (2, 4)
TOY_BIN_APPROACH = (3, 4)
NESTING_APPROACH = (9, 1)
ENRICH_APPROACH = (7, 4)
FEED_APPROACH = (7, 3)

POLLY_FEED_BOWL = (8, 3)
POLLY_NESTING = (9, 2)
POLLY_ENRICHMENT = (7, 5)

DISH_APPROACH = (8, 5)
PUSH_POP_DISH = (8, 6)

TEXT_CANDIDATES = (
    "bevy_ui::widget::text::Text",
    "bevy_text::Text",
    "bevy::prelude::Text",
)

# Polly wander idle is 2s; wait through several cycles before giving up.
BLOCK_RETRY_TIMEOUT_S = 8.0
BLOCK_RETRY_POLL_S = 0.5


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


def follow(
    directions: list[str],
    expect: tuple[int, int],
    *,
    retry_blocked: bool = False,
) -> tuple[int, int] | None:
    print(f"  follow {directions} → expect {expect} from {player_tile()}", flush=True)
    pos = player_tile()
    for direction in directions:
        before = pos
        pos = step(direction)
        if pos == before:
            if retry_blocked:
                deadline = time.time() + BLOCK_RETRY_TIMEOUT_S
                while pos == before and time.time() < deadline:
                    print(f"  blocked at {pos}; retrying {direction}", flush=True)
                    time.sleep(BLOCK_RETRY_POLL_S)
                    pos = step(direction)
            else:
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


def satchel_has(needle: str) -> bool:
    return needle in str(satchel_slots())


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


def animal_stats(animal: str) -> dict | None:
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
            stats = components.get("alveus_stats::AnimalStats", {})
            return stats if isinstance(stats, dict) else None
    return None


def animal_stat_value(animal: str, key: str) -> int | None:
    stats = animal_stats(animal)
    if not stats or key not in stats:
        return None
    return int(stats[key])


def enclosure_cleanliness(enclosure: str) -> int | None:
    result = rpc(
        "world.query",
        {
            "data": {
                "components": [
                    "alveus_types::EnclosureId",
                    "alveus_stats::EnclosureStats",
                ],
                "has": [],
            },
            "filter": {"with": ["alveus_types::EnclosureId"]},
        },
    )
    for row in result or []:
        components = row.get("components", {})
        enc = components.get("alveus_types::EnclosureId")
        if enc == enclosure or (
            isinstance(enc, dict) and enc.get(":variant") == enclosure
        ):
            stats = components.get("alveus_stats::EnclosureStats", {})
            return int(stats.get("cleanliness", 0))
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


def start_gameplay_at_overview_spawn() -> None:
    trigger_game("SkipSplash")
    time.sleep(0.3)
    trigger_game("Play")
    time.sleep(2.0)
    if player_tile() != OVERVIEW_SPAWN:
        raise RuntimeError(f"expected fresh overview spawn, got {player_tile()}")


def enter_nutrition_house() -> None:
    if walk_overview_counts(NAV_UP, NAV_RIGHT_NUTRITION, NUTRITION_ENTRANCE) != NUTRITION_ENTRANCE:
        raise RuntimeError("could not reach Nutrition House entrance")
    if "NutritionHouse" not in player_entrance():
        raise RuntimeError(f"wrong entrance: {player_entrance()}")
    trigger_game("EnterBuilding")
    time.sleep(2.0)
