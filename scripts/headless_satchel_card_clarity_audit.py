#!/usr/bin/env python3
"""Audit that both satchel slots remain visible after a real player care action.

Actions are performed exclusively through GameCommand. Run against a fresh
realtime headless server and stop the server afterward.
"""

from __future__ import annotations

import sys

from headless_care_feedback_audit import (
    BASE,
    REQUIRE_SERVER,
    run_player_feed_flow,
    wait_for_http,
)


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

    failures = []
    slots = result["slots"]
    if not isinstance(slots, list) or len(slots) != 2:
        failures.append(f"expected two satchel slots, got {slots!r}")

    satchel_text = next(
        (
            text
            for text in result["texts"]
            if "Slot 1:" in text and "Slot 2:" in text
        ),
        None,
    )
    if satchel_text is None:
        failures.append(f"two-slot HUD text not found: {result['texts']!r}")
    if result["pickup"] is not None:
        failures.append(f"care outcome leaked into inventory channel: {result['pickup']!r}")

    if failures:
        print("FAIL:", file=sys.stderr)
        for failure in failures:
            print(f"  - {failure}", file=sys.stderr)
        return 1

    print(f"PASS: both satchel slots remain visible after feeding: {satchel_text!r}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
