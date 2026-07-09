#!/usr/bin/env python3
"""Quick BRP smoke: SkipSplash → Play → query player tile with new type paths."""

import json
import time
import urllib.request

BASE = "http://127.0.0.1:15702/"
EVENT = "alveus_headless::command::GameCommand"


def rpc(method, params=None):
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


def trigger(value):
    rpc("world.trigger_event", {"event": EVENT, "value": value})


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


def main():
    discover = rpc("rpc.discover")
    methods = str(discover)
    if "world.trigger_event" not in methods and discover is not None:
        raise SystemExit(f"unexpected rpc.discover: {discover!r}")

    trigger("SkipSplash")
    time.sleep(0.5)
    trigger("Play")
    time.sleep(2.0)

    for _ in range(40):
        tile = player_tile()
        if tile is not None:
            upkeep = rpc(
                "world.get_resources",
                {"resource": "alveus_stats::SanctuaryUpkeep"},
            )
            print(f"ok: player at {tile}, upkeep={upkeep}")
            return
        time.sleep(0.25)

    raise SystemExit("player never spawned")


if __name__ == "__main__":
    main()
