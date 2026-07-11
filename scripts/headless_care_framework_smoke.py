#!/usr/bin/env python3
"""Smoke notes for Epic 1 care framework.

Menu / mini-chore / enrich paths are covered by Rust tests
(`tests/care_interaction_tests.rs`, `tests/interaction_tests.rs`).
This script only documents the BRP satchel shape after Epic 1
(two `slots` instead of a single `item`).

Live fridge menu → Push Pop feed: `scripts/headless_feed_push_pop_demo.py`.
Polly prep → feed → enrich: `scripts/headless_polly_care_demo.py`.
"""

from __future__ import annotations

import json
import sys
import urllib.request

BASE = "http://127.0.0.1:15702/"


def rpc(method, params=None):
    body = {"jsonrpc": "2.0", "id": 1, "method": method}
    if params is not None:
        body["params"] = params
    req = urllib.request.Request(
        BASE,
        data=json.dumps(body).encode(),
        headers={"Content-Type": "application/json"},
    )
    out = json.load(urllib.request.urlopen(req, timeout=5))
    if "error" in out:
        raise RuntimeError(out["error"])
    return out.get("result")


def main() -> int:
    try:
        res = rpc(
            "world.get_resources",
            {"resource": "alveus_interaction::PlayerSatchel"},
        )
    except Exception as exc:
        print(f"skip: headless server not reachable ({exc})", file=sys.stderr)
        return 0

    value = res.get("value", {}) if isinstance(res, dict) else {}
    slots = value.get("slots")
    if not isinstance(slots, list) or len(slots) != 2:
        print(f"FAIL: expected 2 satchel slots, got {slots!r}", file=sys.stderr)
        return 1
    print(f"ok: PlayerSatchel.slots={slots}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
