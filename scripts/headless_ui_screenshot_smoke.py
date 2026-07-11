#!/usr/bin/env python3
"""Smoke: headless Screenshot captures composed world + UI frames.

Requires a realtime headless server, e.g.:

  cargo run --features headless -- --headless --realtime --port 15702 --no-stdio

Then:

  python3 scripts/headless_ui_screenshot_smoke.py

Fails immediately if BRP is unreachable. Writes PNGs under screenshots/.
Visual pixel checks need a wgpu adapter; ECS queries remain logic truth.
Stop the server afterward (do not leave it autosaving save.ron).
"""

from __future__ import annotations

import json
import os
import sys
import time
import urllib.error
import urllib.request

BASE = os.environ.get("ALVEUS_BRP_URL", "http://127.0.0.1:15702/")
EVENT = "alveus_headless::command::GameCommand"
REPO_ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), ".."))
SCREENSHOT_DIR = os.path.join(REPO_ROOT, "screenshots")
GAMEPLAY_PNG = os.path.join(SCREENSHOT_DIR, "ui_smoke_gameplay.png")
MENU_PNG = os.path.join(SCREENSHOT_DIR, "ui_smoke_pause_menu.png")


def rpc(method, params=None):
    body = {"jsonrpc": "2.0", "id": 1, "method": method}
    if params is not None:
        body["params"] = params
    req = urllib.request.Request(
        BASE,
        data=json.dumps(body).encode(),
        headers={"Content-Type": "application/json"},
    )
    try:
        with urllib.request.urlopen(req, timeout=10) as resp:
            out = json.load(resp)
    except urllib.error.URLError as exc:
        raise SystemExit(
            f"BRP server unavailable at {BASE}: {exc}\n"
            "Start: cargo run --features headless -- "
            "--headless --realtime --port 15702 --no-stdio"
        ) from exc
    if "error" in out:
        raise RuntimeError(out["error"])
    return out.get("result")


def trigger(value):
    rpc("world.trigger_event", {"event": EVENT, "value": value})


def get_resource(type_path: str):
    return rpc("world.get_resources", {"resource": type_path})


def screen_state():
    res = get_resource("bevy_state::state::resources::State<alveus_app::Screen>")
    if not res:
        return None
    # Reflect shape varies; accept common wrappers.
    if isinstance(res, dict):
        if "0" in res:
            inner = res["0"]
            if isinstance(inner, str):
                return inner
            if isinstance(inner, dict) and "0" in inner:
                return inner["0"]
        for key in ("value", "state"):
            if key in res:
                return res[key]
    return res


def wait_until(predicate, timeout_s=30.0, interval=0.25, label="condition"):
    deadline = time.time() + timeout_s
    while time.time() < deadline:
        if predicate():
            return
        time.sleep(interval)
    raise SystemExit(f"timed out waiting for {label}")


def wait_for_file(path: str, timeout_s=10.0):
    deadline = time.time() + timeout_s
    while time.time() < deadline:
        if os.path.isfile(path) and os.path.getsize(path) > 0:
            return
        time.sleep(0.1)
    raise SystemExit(f"screenshot not written: {path}")


def capture(path: str):
    if os.path.exists(path):
        os.remove(path)
    trigger({"Screenshot": {"path": path}})
    # Async save — wait well past two frames in realtime.
    time.sleep(0.5)
    wait_for_file(path)


def main():
    os.makedirs(SCREENSHOT_DIR, exist_ok=True)

    # Connectivity probe — fail fast when the server is down.
    rpc("rpc.discover")

    trigger("SkipSplash")
    time.sleep(0.4)
    trigger("Play")

    wait_until(
        lambda: screen_state() in ("Gameplay", {"Gameplay": None})
        or str(screen_state()).endswith("Gameplay"),
        label="Screen::Gameplay",
    )

    # Structured HUD/satchel presence (logic truth).
    satchel = get_resource("alveus_interaction::PlayerSatchel")
    if satchel is None:
        raise SystemExit("PlayerSatchel missing on Gameplay")

    capture(GAMEPLAY_PNG)
    print(f"wrote gameplay frame: {GAMEPLAY_PNG} ({os.path.getsize(GAMEPLAY_PNG)} bytes)")

    trigger("PauseToggle")
    time.sleep(0.4)
    capture(MENU_PNG)
    print(f"wrote pause/menu frame: {MENU_PNG} ({os.path.getsize(MENU_PNG)} bytes)")

    print(
        "ok: screenshots saved. Inspect PNGs for HUD/menu overlay; "
        "ECS resources remain authoritative for logic."
    )
    print("Remember to stop the headless server (pgrep -af alveus-idle-cli).")
    return 0


if __name__ == "__main__":
    sys.exit(main())
