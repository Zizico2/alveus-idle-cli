#!/usr/bin/env python3
"""Exercise BSN menus, Ready initialization, and composed headless rendering.

Run from any directory: python3 scripts/headless_bevy20_smoke.py
Requires a wgpu adapter. Starts/stops its own realtime BRP server and runs in a
temporary save directory, so the developer's save.ron is never touched.
"""

import json
import os
from pathlib import Path
import signal
import socket
import subprocess
import sys
import tempfile
import time
import tomllib
import urllib.request

sys.dont_write_bytecode = True
import headless_ui_screenshot_smoke as smoke

REPO = Path(__file__).resolve().parents[1]
TEXT = "bevy_ui::widget::text::Text"


def text_labels():
    rows = smoke.rpc("world.query", {"data": {"components": [TEXT]}})
    labels = []
    for row in rows:
        value = row["components"][TEXT]
        labels.append(value.get("0", value) if isinstance(value, dict) else value)
    return labels


def capture_standalone(path):
    # Standalone title menus have buttons/text but no gameplay HUD panel.
    path.unlink(missing_ok=True)
    smoke.trigger({"Screenshot": {"path": str(path)}})
    time.sleep(0.5)
    smoke.wait_for_file(str(path))
    width, height, rows = smoke.read_png_rgb(str(path))
    blue = bright = 0
    for row in rows:
        for offset in range(0, len(row), 3):
            r, g, b = row[offset:offset + 3]
            blue += smoke._is_menu_blue(r, g, b)
            bright += r > 180 and g > 180 and b > 150
    assert blue / (width * height) >= smoke.MIN_MENU_BLUE_FRAC, "missing menu buttons"
    assert bright >= 100, "missing menu text"
    print(f"Standalone UI captured: {path.name} ({blue} button pixels, {bright} text pixels)")


def capture_gameplay(path):
    # GPU pipelines can finish after the ECS reports Gameplay. Require world
    # pixels outside the HUD as well as the composed UI checks.
    for _ in range(20):
        smoke.capture(str(path))
        width, height, rows = smoke.read_png_rgb(str(path))
        colored = 0
        for row in rows:
            for offset in range(0, (width * 2 // 3) * 3, 3):
                pixel = row[offset:offset + 3]
                colored += max(pixel) - min(pixel) > 40 and max(pixel) > 60
        if colored > width * height * 0.01:
            return
        time.sleep(0.25)
    raise AssertionError("World geometry did not render outside the HUD")


def main():
    with socket.socket() as listener:
        listener.bind(("127.0.0.1", 0))
        port = listener.getsockname()[1]
    smoke.BASE = f"http://127.0.0.1:{port}/"
    command = ["rtk", "cargo", "run", "--locked", "--manifest-path", str(REPO / "Cargo.toml")]
    config = REPO / ".cargo/config.toml"
    if config.exists():
        command += ["--config", str(config)]
    command += ["--profile", "ci", "--features", "headless", "--", "--headless",
                "--realtime", "--port", str(port), "--no-stdio"]
    toolchain = tomllib.loads((REPO / "rust-toolchain.toml").read_text())["toolchain"]["channel"]
    environment = dict(os.environ, BEVY_ASSET_ROOT=str(REPO), RUSTUP_TOOLCHAIN=toolchain)
    output = REPO / "screenshots"
    output.mkdir(exist_ok=True)
    log_path = output / "bevy20_smoke.log"
    with tempfile.TemporaryDirectory(prefix="alveus-bevy20-") as save_dir, log_path.open("w") as log:
        server = subprocess.Popen(command, cwd=save_dir, env=environment, stdout=log,
                                  stderr=subprocess.STDOUT, start_new_session=True)
        try:
            deadline = time.monotonic() + 300
            while True:
                if server.poll() is not None:
                    raise RuntimeError(f"Server exited; see {log_path}")
                try:
                    request = urllib.request.Request(smoke.BASE, data=json.dumps({
                        "jsonrpc": "2.0", "id": 1, "method": "rpc.discover"
                    }).encode(), headers={"Content-Type": "application/json"})
                    with urllib.request.urlopen(request, timeout=1):
                        break
                except (OSError, TimeoutError):
                    if time.monotonic() >= deadline:
                        raise RuntimeError(f"Server startup timed out; see {log_path}")
                    time.sleep(0.25)

            smoke.trigger("SkipSplash")
            time.sleep(0.4)
            smoke.trigger("OpenSettings")
            time.sleep(0.4)
            assert "100%" in [text.strip() for text in text_labels()], "Ready initializes volume"
            smoke.trigger({"AdjustVolume": {"delta": -0.25}})
            time.sleep(0.2)
            assert "75%" in [text.strip() for text in text_labels()]
            smoke.trigger("Back")
            time.sleep(0.2)
            smoke.trigger("OpenSettings")
            time.sleep(0.3)
            assert "75%" in [text.strip() for text in text_labels()], "Ready initializes on reopening"
            capture_standalone(output / "bevy20_settings.png")

            smoke.trigger("Back")
            time.sleep(0.2)
            smoke.trigger("OpenCredits")
            time.sleep(0.4)
            assert "Created by" in text_labels()
            capture_standalone(output / "bevy20_credits.png")

            smoke.trigger("Back")
            time.sleep(0.2)
            smoke.trigger("Play")
            smoke.wait_until(smoke.in_gameplay, label="Gameplay")
            tile = smoke.player_tile()
            assert tile is not None
            print(f"Gameplay ready at authoritative player tile {tile}")
            capture_gameplay(output / "bevy20_gameplay.png")
            smoke.trigger("PauseToggle")
            time.sleep(0.3)
            smoke.capture(str(output / "bevy20_pause.png"), expect_menu=True)
            print("PASS: BSN settings, Ready reopening, credits, gameplay HUD, and pause menu")
        finally:
            if server.poll() is None:
                os.killpg(server.pid, signal.SIGTERM)
            try:
                server.wait(timeout=5)
            except subprocess.TimeoutExpired:
                os.killpg(server.pid, signal.SIGKILL)
                server.wait()


if __name__ == "__main__":
    main()
