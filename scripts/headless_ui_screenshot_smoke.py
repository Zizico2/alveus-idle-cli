#!/usr/bin/env python3
"""Smoke: headless Screenshot must capture composed world + UI frames.

Requires a realtime headless server, e.g.:

  cargo run --features headless -- --headless --realtime --port 15702 --no-stdio

Then:

  python3 scripts/headless_ui_screenshot_smoke.py

Fails immediately if BRP is unreachable. Writes PNGs under screenshots/ and
**asserts** that each PNG contains UI overlay pixels (dark HUD chrome and
teal accents for gameplay; blue menu chrome for pause). A world-only
regression fails this script — it is not a manual-inspect-only artifact dump.

Pixel checks need a wgpu adapter (GPU or lavapipe). ECS queries remain logic
truth. Stop the server afterward (do not leave it autosaving save.ron).
"""

from __future__ import annotations

import json
import os
import struct
import sys
import time
import urllib.error
import urllib.request
import zlib

BASE = os.environ.get("ALVEUS_BRP_URL", "http://127.0.0.1:15702/")
EVENT = "alveus_headless::command::GameCommand"
REPO_ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), ".."))
SCREENSHOT_DIR = os.path.join(REPO_ROOT, "screenshots")
GAMEPLAY_PNG = os.path.join(SCREENSHOT_DIR, "ui_smoke_gameplay.png")
MENU_PNG = os.path.join(SCREENSHOT_DIR, "ui_smoke_pause_menu.png")
# Registered in register_types — requires a server built with that registration.
SCREEN_STATE = "bevy_state::state::resources::State<alveus_app::Screen>"

# Sample stride keeps runtime low; thresholds are fractions of sampled pixels.
SAMPLE_STRIDE = 2
# Dark glassmorphic HUD panels (approx. srgb 0.08–0.12) after compositing.
MIN_DARK_PANEL_FRAC = 0.05
# Teal/green HUD accents (progress fills / borders ~ srgb(0.2, 0.9, 0.6)).
MIN_TEAL_FRAC = 0.0005
# Pause menu primary buttons are saturated blue.
MIN_MENU_BLUE_FRAC = 0.01


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


def player_tile():
    res = rpc(
        "world.query",
        {
            "data": {
                "components": ["alveus_components::CurrentTilePosition"],
                "has": [],
            },
            "filter": {"with": ["alveus_components::Player"]},
        },
    )
    row = (res or [None])[0]
    if not row:
        return None
    pos = row["components"]["alveus_components::CurrentTilePosition"]
    inner = pos["0"] if isinstance(pos, dict) and "0" in pos else pos
    return int(inner["x"]), int(inner["y"])


def unwrap_state(res):
    """Normalize BRP State<T> / resource envelopes to a plain variant name."""
    if res is None:
        return None
    if isinstance(res, str):
        return res
    if isinstance(res, dict):
        if "value" in res:
            return unwrap_state(res["value"])
        if "0" in res:
            return unwrap_state(res["0"])
        if len(res) == 1:
            (key,) = res.keys()
            return key
    return res


def screen_state():
    try:
        return unwrap_state(get_resource(SCREEN_STATE))
    except RuntimeError as exc:
        # Older headless builds may lack State<Screen> registration.
        if "Unknown resource type" in str(exc) or "isn't reflectable" in str(exc):
            return None
        raise


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


def _paeth(a: int, b: int, c: int) -> int:
    p = a + b - c
    pa, pb, pc = abs(p - a), abs(p - b), abs(p - c)
    if pa <= pb and pa <= pc:
        return a
    if pb <= pc:
        return b
    return c


def read_png_rgb(path: str) -> tuple[int, int, list[bytes]]:
    """Decode an 8-bit RGB/RGBA PNG to raw row bytes (RGB only) via stdlib."""
    with open(path, "rb") as handle:
        if handle.read(8) != b"\x89PNG\r\n\x1a\n":
            raise SystemExit(f"not a PNG: {path}")
        width = height = None
        color_type = None
        idat = b""
        while True:
            length = struct.unpack(">I", handle.read(4))[0]
            chunk = handle.read(4)
            data = handle.read(length)
            handle.read(4)  # CRC
            if chunk == b"IHDR":
                width, height, bit_depth, color_type = struct.unpack(">IIBB", data[:10])
                if bit_depth != 8 or color_type not in (2, 6):
                    raise SystemExit(
                        f"unsupported PNG format in {path}: "
                        f"bit_depth={bit_depth} color_type={color_type}"
                    )
            elif chunk == b"IDAT":
                idat += data
            elif chunk == b"IEND":
                break
    assert width is not None and height is not None and color_type is not None
    raw = zlib.decompress(idat)
    bpp = 3 if color_type == 2 else 4
    stride = width * bpp
    rows: list[bytes] = []
    prev = bytearray(stride)
    offset = 0
    for _ in range(height):
        filt = raw[offset]
        offset += 1
        row = bytearray(raw[offset : offset + stride])
        offset += stride
        if filt == 1:
            for x in range(stride):
                left = row[x - bpp] if x >= bpp else 0
                row[x] = (row[x] + left) & 255
        elif filt == 2:
            for x in range(stride):
                row[x] = (row[x] + prev[x]) & 255
        elif filt == 3:
            for x in range(stride):
                left = row[x - bpp] if x >= bpp else 0
                row[x] = (row[x] + ((left + prev[x]) // 2)) & 255
        elif filt == 4:
            for x in range(stride):
                left = row[x - bpp] if x >= bpp else 0
                up = prev[x]
                up_left = prev[x - bpp] if x >= bpp else 0
                row[x] = (row[x] + _paeth(left, up, up_left)) & 255
        elif filt != 0:
            raise SystemExit(f"unsupported PNG filter {filt} in {path}")
        # Store RGB only for sampling.
        if bpp == 3:
            rows.append(bytes(row))
        else:
            rgb = bytearray(width * 3)
            for i in range(width):
                src = i * 4
                dst = i * 3
                rgb[dst : dst + 3] = row[src : src + 3]
            rows.append(bytes(rgb))
        prev = row
    return width, height, rows


def _is_letterbox(r: int, g: int, b: int) -> bool:
    # Flat dark padding outside the composed game view.
    if r + g + b < 15:
        return True
    return abs(r - g) < 8 and abs(g - b) < 8 and 30 <= r <= 55


def _is_dark_panel(r: int, g: int, b: int) -> bool:
    """Composited HUD panels are dark charcoal alpha-blended over the world.

    After blending they often read as muted green-tinted darks (e.g. 36,77,40),
    not pure gray — so reject bright grass but accept low-luminance chrome.
    """
    lum = (r + g + b) / 3.0
    if lum < 8 or lum > 80:
        return False
    if max(r, g, b) > 110:
        return False
    # Saturated grass / foliage, not panel chrome.
    if g > r + 45 and g > b + 45 and g > 95:
        return False
    return True


def _is_teal_accent(r: int, g: int, b: int) -> bool:
    return 20 <= r <= 100 and g >= 150 and 80 <= b <= 220 and g > r + 40


def _is_menu_blue(r: int, g: int, b: int) -> bool:
    return b > 120 and b > r + 25 and b > g


def assert_ui_captured(path: str, *, expect_menu: bool) -> None:
    """Fail if the PNG looks like a world-only (no HUD/menu) capture."""
    width, height, rows = read_png_rgb(path)
    content: list[tuple[int, int]] = []
    for y in range(0, height, SAMPLE_STRIDE):
        row = rows[y]
        for x in range(0, width, SAMPLE_STRIDE):
            o = x * 3
            r, g, b = row[o], row[o + 1], row[o + 2]
            if _is_letterbox(r, g, b):
                continue
            content.append((x, y))
    if len(content) < 100:
        raise SystemExit(f"{path}: too little non-letterbox content to assert UI")

    xs = [p[0] for p in content]
    ys = [p[1] for p in content]
    x0, x1, y0, y1 = min(xs), max(xs), min(ys), max(ys)
    # HUD chrome lives on the right; pause buttons are centered — sample both.
    mid = x0 + (x1 - x0) * 2 // 3
    dark = teal = blue = right_samples = full_samples = 0
    for y in range(y0, y1 + 1, SAMPLE_STRIDE):
        row = rows[y]
        for x in range(x0, x1 + 1, SAMPLE_STRIDE):
            o = x * 3
            r, g, b = row[o], row[o + 1], row[o + 2]
            if _is_letterbox(r, g, b):
                continue
            full_samples += 1
            if _is_menu_blue(r, g, b):
                blue += 1
            if x < mid:
                continue
            right_samples += 1
            if _is_dark_panel(r, g, b):
                dark += 1
            if _is_teal_accent(r, g, b):
                teal += 1
    if right_samples < 50 or full_samples < 50:
        raise SystemExit(f"{path}: content too sparse for UI assert")

    dark_frac = dark / right_samples
    teal_frac = teal / right_samples
    blue_frac = blue / full_samples
    print(
        f"ui assert {os.path.basename(path)}: right={right_samples} full={full_samples} "
        f"dark_panel={dark_frac:.3f} teal={teal_frac:.4f} blue={blue_frac:.4f}"
    )

    if dark_frac < MIN_DARK_PANEL_FRAC:
        raise SystemExit(
            f"{path}: missing dark HUD/menu chrome "
            f"(dark_panel={dark_frac:.3f} < {MIN_DARK_PANEL_FRAC}). "
            "Likely a world-only Screenshot regression."
        )
    if expect_menu:
        if blue_frac < MIN_MENU_BLUE_FRAC:
            raise SystemExit(
                f"{path}: missing pause-menu blue chrome "
                f"(blue={blue_frac:.4f} < {MIN_MENU_BLUE_FRAC}). "
                "Likely a world-only or menu-not-rendered regression."
            )
    elif teal_frac < MIN_TEAL_FRAC:
        raise SystemExit(
            f"{path}: missing HUD teal accents "
            f"(teal={teal_frac:.4f} < {MIN_TEAL_FRAC}). "
            "Likely a world-only Screenshot regression."
        )


def capture(path: str, *, expect_menu: bool = False):
    if os.path.exists(path):
        os.remove(path)
    trigger({"Screenshot": {"path": path}})
    # Async save — wait well past two frames in realtime.
    time.sleep(0.5)
    wait_for_file(path)
    assert_ui_captured(path, expect_menu=expect_menu)


def in_gameplay() -> bool:
    # Authoritative: player exists on the overview (or interior) map.
    if player_tile() is not None:
        return True
    # Optional: State<Screen> when the running binary registered it.
    screen = screen_state()
    return screen in ("Gameplay", {"Gameplay": None}) or (
        isinstance(screen, str) and screen.endswith("Gameplay")
    )


def main():
    os.makedirs(SCREENSHOT_DIR, exist_ok=True)

    # Connectivity probe — fail fast when the server is down.
    rpc("rpc.discover")

    trigger("SkipSplash")
    time.sleep(0.4)
    trigger("Play")

    wait_until(in_gameplay, label="Gameplay (player tile / Screen::Gameplay)")

    # Structured HUD/satchel presence (logic truth).
    satchel = get_resource("alveus_interaction::PlayerSatchel")
    if satchel is None:
        raise SystemExit("PlayerSatchel missing on Gameplay")

    tile = player_tile()
    print(f"gameplay ready: player_tile={tile}, screen={screen_state()!r}")

    capture(GAMEPLAY_PNG, expect_menu=False)
    print(f"wrote gameplay frame: {GAMEPLAY_PNG} ({os.path.getsize(GAMEPLAY_PNG)} bytes)")

    trigger("PauseToggle")
    time.sleep(0.4)
    capture(MENU_PNG, expect_menu=True)
    print(f"wrote pause/menu frame: {MENU_PNG} ({os.path.getsize(MENU_PNG)} bytes)")

    print(
        "ok: screenshots contain UI overlay pixels "
        "(dark HUD chrome + teal accents / menu blue)."
    )
    print("Remember to stop the headless server (pgrep -af alveus-idle-cli).")
    return 0


if __name__ == "__main__":
    sys.exit(main())
