#!/usr/bin/env python3
"""Playtest: Push Pop Enclosure — pick up poops, empty wheelbarrow at overview compost bin."""

from __future__ import annotations

import json
import sys
import time
import urllib.error
import urllib.request

PORT = 15702
BASE = f"http://127.0.0.1:{PORT}/"
EVENT = "alveus_idle_cli::headless::command::GameCommand"

PUSH_POP_ENTRANCE = (39, 12)
COMPOST_BIN = (3, 0)


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
            "components": ["alveus_idle_cli::components::CurrentTilePosition"],
            "has": [],
        },
        "filter": {"with": ["alveus_idle_cli::demo::player::Player"]},
    })
    row = (res or [None])[0]
    if not row:
        return None
    raw = row.get("components", {}).get(
        "alveus_idle_cli::components::CurrentTilePosition"
    )
    return parse_tile_position(raw)


def enclosure_cleanliness() -> int | None:
    res = rpc("world.query", {
        "data": {
            "components": [
                "alveus_idle_cli::stats::EnclosureId",
                "alveus_idle_cli::stats::EnclosureStats",
            ],
            "has": [],
        },
        "filter": {"with": ["alveus_idle_cli::stats::EnclosureId"]},
    })
    for row in res or []:
        comps = row.get("components", {})
        enc = comps.get("alveus_idle_cli::stats::EnclosureId")
        if enc == "PushPopEnclosure" or (
            isinstance(enc, dict) and enc.get(":variant") == "PushPopEnclosure"
        ):
            stats = comps.get("alveus_idle_cli::stats::EnclosureStats", {})
            return int(stats.get("cleanliness", 0))
    return None


def wheelbarrow_count() -> int:
    res = rpc("world.get_resources", {
        "resource": "alveus_idle_cli::cleaning::PoopWheelbarrow",
    })
    wb = (res or {}).get("value", {})
    poops = wb.get("poops", [])
    return len(poops)


def poop_tile_count() -> int:
    res = rpc("world.query", {
        "data": {
            "components": ["alveus_idle_cli::collision::DynamicObstacleTiles"],
            "has": [],
        },
        "filter": {"with": ["alveus_idle_cli::stats::EnclosureId"]},
    })
    for row in res or []:
        comps = row.get("components", {})
        enc = comps.get("alveus_idle_cli::stats::EnclosureId")
        if enc == "PushPopEnclosure" or (
            isinstance(enc, dict) and enc.get(":variant") == "PushPopEnclosure"
        ):
            tiles = comps.get("alveus_idle_cli::collision::DynamicObstacleTiles", [])
            if isinstance(tiles, dict) and "0" in tiles:
                tiles = tiles["0"]
            return len(tiles or [])
    return 0


def step(direction: str, hold: float = 0.35) -> tuple[int, int] | None:
    before = player_tile()
    trigger({"Move": direction})
    time.sleep(hold)
    trigger("MoveStop")
    time.sleep(0.05)
    after = player_tile()
    if after == before:
        print(f"blocked: still at {after} after Move {direction}")
    return after


def navigate_to(target: tuple[int, int], max_steps: int = 40) -> tuple[int, int] | None:
    current = player_tile()
    if current is None:
        return None
    for _ in range(max_steps):
        current = player_tile()
        if current == target:
            return current
        tx, ty = target
        cx, cy = current
        if cx < tx:
            current = step("Right")
        elif cx > tx:
            current = step("Left")
        elif cy < ty:
            current = step("Up")
        elif cy > ty:
            current = step("Down")
        if current == target:
            return current
    return player_tile()


def main() -> int:
    wait_for_http()

    trigger({"AdvanceTime": {"hours": 12.0}})
    time.sleep(0.1)

    print(f"Before enter: poops on floor={poop_tile_count()}, cleanliness={enclosure_cleanliness()}")

    navigate_to(PUSH_POP_ENTRANCE)
    trigger("EnterBuilding")
    time.sleep(0.2)
    print(f"In enclosure at {player_tile()}, wheelbarrow={wheelbarrow_count()}")

    initial_poops = poop_tile_count()
    clean_before_pickup = enclosure_cleanliness()
    for i in range(initial_poops + 2):
        trigger("Interact")
        time.sleep(0.05)
        wb = wheelbarrow_count()
        remaining = poop_tile_count()
        clean = enclosure_cleanliness()
        print(f"  interact {i + 1}: wheelbarrow={wb}, poops left={remaining}, cleanliness={clean}")
        if wb >= initial_poops or remaining == 0:
            break

    clean_after_pickup = enclosure_cleanliness()
    if (
        clean_before_pickup is not None
        and clean_after_pickup is not None
        and clean_after_pickup <= clean_before_pickup
    ):
        print("FAIL: cleanliness should increase when poops are picked up", file=sys.stderr)
        return 1

    trigger("ExitRoom")
    time.sleep(0.1)
    print(f"Back on overview at {player_tile()}, wheelbarrow={wheelbarrow_count()}")

    navigate_to(COMPOST_BIN)
    clean_before = enclosure_cleanliness()
    trigger("Interact")
    time.sleep(0.05)
    clean_after = enclosure_cleanliness()
    wb_after = wheelbarrow_count()
    print(f"After dump: cleanliness {clean_before} -> {clean_after}, wheelbarrow={wb_after}")

    if wb_after != 0:
        print("FAIL: wheelbarrow should be empty after dump", file=sys.stderr)
        return 1
    if clean_after != clean_before:
        print("FAIL: dumping the wheelbarrow should not change cleanliness", file=sys.stderr)
        return 1

    print("OK: cleaning loop completed")
    return 0


if __name__ == "__main__":
    sys.exit(main())
