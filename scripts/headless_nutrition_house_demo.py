#!/usr/bin/env python3
"""Drive headless alveus-idle-cli into the Nutrition House and capture a screenshot."""

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
SCREENSHOT = SCREENSHOTS / "nutrition_house_screenshot.png"
# Example constants — verify in src/ before relying on these (map/room config can change).
# Nutrition House entrance: see AGENTS.md §4 table; derived from map + entrance.rs logs.
NAV_RIGHT = 33
NAV_UP = 12


def rpc(method: str, params=None, req_id: int = 1) -> dict:
    payload = {"jsonrpc": "2.0", "method": method, "id": req_id}
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


def wait_for_http(timeout_s: float = 60.0) -> None:
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


def main() -> int:
    print("Waiting for BRP HTTP…", flush=True)
    wait_for_http()

    print("Skipping splash (if still on splash)…", flush=True)
    trigger("SkipSplash")
    time.sleep(0.5)

    print("Starting game…", flush=True)
    trigger("Play")
    time.sleep(3.0)

    print(f"Walking right × {NAV_RIGHT}…", flush=True)
    for i in range(NAV_RIGHT):
        move_dir("Right")
        if (i + 1) % 10 == 0:
            print(f"  …{i + 1}/{NAV_RIGHT}", flush=True)

    print(f"Walking up × {NAV_UP}…", flush=True)
    for i in range(NAV_UP):
        move_dir("Up")
        if (i + 1) % 10 == 0:
            print(f"  …{i + 1}/{NAV_UP}", flush=True)

    print("Entering Nutrition House…", flush=True)
    trigger("EnterBuilding")
    time.sleep(3.0)

    print(f"Taking screenshot → {SCREENSHOT}", flush=True)
    SCREENSHOT.parent.mkdir(parents=True, exist_ok=True)
    trigger({"Screenshot": {"path": str(SCREENSHOT)}})
    time.sleep(2.0)

    print("Done.", flush=True)
    return 0


if __name__ == "__main__":
    sys.exit(main())
