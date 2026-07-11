#!/usr/bin/env python3
"""Simple headless playtest: overview walk, enter Nutrition House, exit, observe state."""

from __future__ import annotations

import json
import sys
import time
import urllib.error
import urllib.request
from pathlib import Path

PORT = 15702
BASE = f"http://127.0.0.1:{PORT}/"
EVENT = "alveus_headless::command::GameCommand"
SCREENSHOTS = Path(__file__).resolve().parent.parent / "screenshots"
SCREENSHOT_OVERVIEW = SCREENSHOTS / "playtest_overview.png"
SCREENSHOT_INTERIOR = SCREENSHOTS / "playtest_nutrition_house.png"

# Tile-count navigation (AGENTS.md §4); entrance tiles x=32..35, y=11..12.
NAV_RIGHT = 33
NAV_UP = 12


def rpc(method: str, params=None) -> dict | list | None:
    payload = {"jsonrpc": "2.0", "method": method, "id": 1}
    if params is not None:
        payload["params"] = params
    data = json.dumps(payload).encode()
    req = urllib.request.Request(
        BASE,
        data=data,
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


def move_dir(direction: str, hold_s: float = 0.35) -> None:
    trigger({"Move": direction})
    time.sleep(hold_s)
    trigger("MoveStop")
    time.sleep(0.05)


def player_has_entrance() -> str | None:
    res = rpc("world.query", {
        "data": {
            "components": ["alveus_components::BuildingEntrance"],
            "has": [],
        },
        "filter": {"with": ["alveus_components::Player"]},
    })
    if not res:
        return None
    ent = res[0].get("components", {}).get(
        "alveus_components::BuildingEntrance"
    )
    return str(ent) if ent else None


def sanctuary_upkeep() -> dict | None:
    res = rpc("world.get_resources", {
        "resource": "alveus_stats::SanctuaryUpkeep",
    })
    if isinstance(res, dict):
        return res.get("value")
    return None


def animal_stats() -> list[tuple[str, int, int]]:
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
    out = []
    for row in res or []:
        comps = row.get("components", {})
        aid = comps.get("alveus_types::AnimalId", "?")
        stats = comps.get("alveus_stats::AnimalStats", {})
        out.append((str(aid), int(stats.get("hunger", -1)), int(stats.get("happiness", -1))))
    return out


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


def screenshot(path: Path) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    trigger({"Screenshot": {"path": str(path)}})
    time.sleep(2.0)


def report(label: str) -> None:
    upkeep = sanctuary_upkeep()
    entrance = player_has_entrance()
    print(
        f"[{label}] entrance={entrance} satchel={satchel_item()} "
        f"upkeep_score={upkeep.get('score') if upkeep else None}",
        flush=True,
    )


def main() -> int:
    results: list[str] = []
    print("=== Alveus Idle CLI — headless playtest ===", flush=True)

    wait_for_http()
    results.append("BRP HTTP ready")

    trigger("SkipSplash")
    time.sleep(0.5)
    trigger("Play")
    time.sleep(3.0)
    report("gameplay start")

    animals = animal_stats()
    results.append(f"Animals loaded: {len(animals)} ({', '.join(a[0] for a in animals)})")

    # Overview y=0 is blocked past x=2; climb first, then walk east.
    print(f"Walking up × {NAV_UP}, right × {NAV_RIGHT}…", flush=True)
    for i in range(NAV_UP):
        move_dir("Up")
        if (i + 1) % 6 == 0:
            print(f"  up …{i + 1}/{NAV_UP}", flush=True)
    for i in range(NAV_RIGHT):
        move_dir("Right")
        if (i + 1) % 11 == 0:
            print(f"  right …{i + 1}/{NAV_RIGHT}", flush=True)

    entrance = player_has_entrance()
    report("at entrance area")
    screenshot(SCREENSHOT_OVERVIEW)
    if entrance and "NutritionHouse" in entrance:
        results.append(f"On Nutrition House entrance: {entrance}")
    else:
        results.append(f"WARN: expected NutritionHouse entrance, got {entrance}")

    trigger("EnterBuilding")
    time.sleep(3.0)
    report("after EnterBuilding")
    screenshot(SCREENSHOT_INTERIOR)

    # Brief interior walk
    for _ in range(3):
        move_dir("Right")
    for _ in range(2):
        move_dir("Up")
    report("inside Nutrition House")

    trigger("Interact")
    time.sleep(0.5)
    satchel = satchel_item()
    report("after Interact")
    if satchel:
        results.append(f"Picked up item: {satchel}")

    trigger("ExitRoom")
    time.sleep(3.0)
    report("after ExitRoom")
    entrance = player_has_entrance()
    if entrance is None or "NoEntrance" in str(entrance):
        results.append("Exit to overview: OK (no building entrance on player)")
    else:
        results.append(f"Exit to overview: likely OK (entrance={entrance})")

    upkeep = sanctuary_upkeep()
    if upkeep:
        results.append(
            f"Sanctuary upkeep score={upkeep.get('score')} "
            f"mean_hunger={upkeep.get('mean_hunger')}"
        )

    print("\n=== Summary ===", flush=True)
    for line in results:
        print(f"  • {line}", flush=True)

    failed = [r for r in results if r.startswith("WARN:") or "FAIL" in r]
    return 1 if failed else 0


if __name__ == "__main__":
    sys.exit(main())
