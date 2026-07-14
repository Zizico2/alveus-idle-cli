#!/usr/bin/env python3
"""BRP audit for player-reachable care feedback (feed / clean / enrich).

Each case uses only GameCommand verbs, walks real interactable routes, and
asserts exact positive/negative toast copy plus the relevant stat/inventory
effect. Cases are isolated so the validation wrapper can restart the server
and save between them:

  python3 scripts/headless_care_feedback_audit.py feed
  python3 scripts/headless_care_feedback_audit.py clean
  python3 scripts/headless_care_feedback_audit.py enrich

Server absence fails by default. Set ALLOW_MISSING_SERVER=1 only for local
optional skips — validation wrappers must never set it.

Run against a fresh realtime headless server, then stop that server afterward:

  cargo run --features headless -- --headless --realtime --port 15702 --no-stdio
"""

from __future__ import annotations

import sys
import time

from brp_driver import (
    ALLOW_MISSING_SERVER,
    BASE,
    DISH_APPROACH,
    ENRICH_APPROACH,
    FRIDGE_APPROACH,
    NESTING_APPROACH,
    POLLY_ENRICHMENT,
    POLLY_FEED_BOWL,
    POLLY_NESTING,
    PUSH_POP_DISH,
    PUSH_POP_ENTRANCE,
    TOY_BIN_APPROACH,
    NAV_RIGHT_TO_PUSH_POP,
    animal_stat_value,
    enclosure_cleanliness,
    enter_nutrition_house,
    follow,
    is_adjacent,
    last_pickup_text,
    player_entrance,
    player_tile,
    satchel_has,
    satchel_slots,
    start_gameplay_at_overview_spawn,
    trigger_game,
    ui_texts,
    wait_for_http,
    walk_overview_counts,
)

SCENARIOS = ("feed", "clean", "enrich")


def run_player_feed_flow() -> dict:
    """Nutrition House greens → Push Pop feed. Used by satchel audit too."""
    start_gameplay_at_overview_spawn()

    trigger_game(
        {
            "WorsenStat": {
                "target": {"Animal": {"id": "PushPop", "stat": "Hunger"}},
                "amount": 600,
            }
        }
    )
    time.sleep(0.2)
    hunger_before = animal_stat_value("PushPop", "hunger")
    happiness_before = animal_stat_value("PushPop", "happiness")
    clean_before = enclosure_cleanliness("PushPopEnclosure")

    enter_nutrition_house()

    # Avoid toy bin (3,5) and seed chest (2,5): (5,2)→(4,2)→(4,6)→(2,6)→(2,7).
    if follow(
        ["Left", "Up", "Up", "Up", "Up", "Left", "Left", "Up"],
        FRIDGE_APPROACH,
        retry_blocked=True,
    ) != FRIDGE_APPROACH:
        raise RuntimeError(f"could not reach diet fridge, at {player_tile()}")
    # Fridge is CareItemPicker: first option RawVeggieTub, second TortoiseLeafyGreens.
    trigger_game("Interact")
    time.sleep(0.3)
    trigger_game({"NavigateListMenu": "Down"})
    time.sleep(0.15)
    trigger_game("Continue")
    time.sleep(0.4)
    if not satchel_has("TortoiseLeafyGreens"):
        raise RuntimeError(f"greens not acquired: {satchel_slots()!r}")

    trigger_game("ExitRoom")
    time.sleep(2.0)
    if walk_overview_counts(0, NAV_RIGHT_TO_PUSH_POP, PUSH_POP_ENTRANCE) != PUSH_POP_ENTRANCE:
        raise RuntimeError("could not reach Push Pop entrance")
    if "PushPop" not in player_entrance():
        raise RuntimeError(f"wrong entrance: {player_entrance()}")
    trigger_game("EnterBuilding")
    time.sleep(2.0)

    if follow(["Up", "Up", "Up", "Right", "Right"], DISH_APPROACH) != DISH_APPROACH:
        pos = player_tile()
        if pos is None or not is_adjacent(pos, PUSH_POP_DISH):
            follow(["Left", "Up"], (7, 6))
            pos = player_tile()
            if pos is None or not is_adjacent(pos, PUSH_POP_DISH):
                raise RuntimeError(f"could not reach Push Pop feeding dish, at {pos}")
    trigger_game("Interact")
    time.sleep(0.35)

    return {
        "animal": "Push Pop",
        "stat_before": hunger_before,
        "stat_after": animal_stat_value("PushPop", "hunger"),
        "unrelated": {
            "happiness": (
                happiness_before,
                animal_stat_value("PushPop", "happiness"),
            ),
            "cleanliness": (
                clean_before,
                enclosure_cleanliness("PushPopEnclosure"),
            ),
        },
        "slots": satchel_slots(),
        "consumed_item": "TortoiseLeafyGreens",
        "pickup": last_pickup_text(),
        "texts": ui_texts(),
        "tile": player_tile(),
    }


def run_player_clean_flow() -> dict:
    """Nutrition House nesting sweep → Cleaned Polly."""
    start_gameplay_at_overview_spawn()

    clean_before = enclosure_cleanliness("NutritionHousePlaypen")
    if clean_before is None:
        raise RuntimeError("could not read NutritionHousePlaypen cleanliness")
    if clean_before >= 900:
        trigger_game(
            {
                "WorsenStat": {
                    "target": {
                        "Enclosure": {
                            "id": "NutritionHousePlaypen",
                            "stat": "Cleanliness",
                        }
                    },
                    "amount": 600,
                }
            }
        )
        time.sleep(0.2)
        clean_before = enclosure_cleanliness("NutritionHousePlaypen")
        if clean_before is None:
            raise RuntimeError("could not read cleanliness after WorsenStat")

    hunger_before = animal_stat_value("Polly", "hunger")
    happiness_before = animal_stat_value("Polly", "happiness")

    enter_nutrition_house()

    # Prefer (9,1): adjacent to nesting (9,2) but not the feed bowl (8,3).
    # (8,2) is adjacent to both and can Interact the wrong station.
    if follow(
        ["Right", "Right", "Right", "Down", "Right"],
        NESTING_APPROACH,
        retry_blocked=True,
    ) != NESTING_APPROACH:
        raise RuntimeError(f"could not reach nesting approach, at {player_tile()}")
    reached = player_tile()
    if reached is None or not is_adjacent(reached, POLLY_NESTING):
        raise RuntimeError(f"{reached} not adjacent to nesting {POLLY_NESTING}")
    if is_adjacent(reached, POLLY_FEED_BOWL):
        raise RuntimeError(f"{reached} is also adjacent to feed bowl; ambiguous Interact")

    trigger_game("Interact")
    time.sleep(0.4)

    return {
        "animal": "Polly",
        "stat_before": clean_before,
        "stat_after": enclosure_cleanliness("NutritionHousePlaypen"),
        "unrelated": {
            "hunger": (hunger_before, animal_stat_value("Polly", "hunger")),
            "happiness": (
                happiness_before,
                animal_stat_value("Polly", "happiness"),
            ),
        },
        "slots": satchel_slots(),
        "consumed_item": None,
        "pickup": last_pickup_text(),
        "texts": ui_texts(),
        "tile": player_tile(),
    }


def run_player_enrich_flow() -> dict:
    """Nutrition House toy bin → enrichment post → Enriched Polly."""
    start_gameplay_at_overview_spawn()

    happiness_before = animal_stat_value("Polly", "happiness")
    if happiness_before is None:
        raise RuntimeError("could not read Polly happiness")
    if happiness_before > 200:
        trigger_game(
            {
                "WorsenStat": {
                    "target": {"Animal": {"id": "Polly", "stat": "Happiness"}},
                    "amount": 600,
                }
            }
        )
        time.sleep(0.2)
        happiness_before = animal_stat_value("Polly", "happiness")
        if happiness_before is None:
            raise RuntimeError("could not read happiness after WorsenStat")

    hunger_before = animal_stat_value("Polly", "hunger")
    clean_before = enclosure_cleanliness("NutritionHousePlaypen")

    enter_nutrition_house()

    # (5,2) → Left×2 → (3,2) → Up×2 → (3,4) toy bin.
    if follow(
        ["Left", "Left", "Up", "Up"],
        TOY_BIN_APPROACH,
        retry_blocked=True,
    ) != TOY_BIN_APPROACH:
        raise RuntimeError(f"could not reach toy bin, at {player_tile()}")
    trigger_game("Interact")
    time.sleep(0.4)
    if not satchel_has("MiniMirror"):
        raise RuntimeError(f"MiniMirror not acquired: {satchel_slots()!r}")

    enrich_pos = follow(
        ["Right", "Right", "Down", "Right", "Right", "Up"],
        ENRICH_APPROACH,
        retry_blocked=True,
    )
    if enrich_pos != ENRICH_APPROACH:
        if player_tile() == (7, 3):
            enrich_pos = follow(["Up", "Right"], (8, 5), retry_blocked=True)
        elif player_tile() == (7, 4):
            enrich_pos = follow(["Right"], (8, 5), retry_blocked=True)
    reached = player_tile()
    if reached is None or not is_adjacent(reached, POLLY_ENRICHMENT):
        raise RuntimeError(
            f"{reached} not adjacent to enrichment {POLLY_ENRICHMENT}"
        )

    trigger_game("Interact")
    time.sleep(0.5)

    return {
        "animal": "Polly",
        "stat_before": happiness_before,
        "stat_after": animal_stat_value("Polly", "happiness"),
        "unrelated": {
            "hunger": (hunger_before, animal_stat_value("Polly", "hunger")),
            "cleanliness": (
                clean_before,
                enclosure_cleanliness("NutritionHousePlaypen"),
            ),
        },
        "slots": satchel_slots(),
        "consumed_item": "MiniMirror",
        "pickup": last_pickup_text(),
        "texts": ui_texts(),
        "tile": player_tile(),
    }


def _assert_unrelated_unchanged(unrelated: dict) -> list[str]:
    failures = []
    for name, pair in unrelated.items():
        before, after = pair
        if before is None or after is None:
            failures.append(f"could not read unrelated {name}: {before} -> {after}")
        elif after != before:
            failures.append(f"unrelated {name} changed: {before} -> {after}")
    return failures


def assert_feed(result: dict) -> list[str]:
    failures = []
    before, after = result["stat_before"], result["stat_after"]
    if before is None or after is None or after <= before:
        failures.append(f"hunger did not improve: {before} -> {after}")
    if result["consumed_item"] and result["consumed_item"] in str(result["slots"]):
        failures.append(f"feeding did not consume greens: {result['slots']!r}")
    expected = f"Fed {result['animal']}"
    care_toast = next((t for t in result["texts"] if expected in t), None)
    if care_toast is None:
        failures.append(f"{expected!r} toast not found: {result['texts']!r}")
    elif "Cleaned" in care_toast or "Enriched" in care_toast:
        failures.append(f"feed toast has wrong verb: {care_toast!r}")
    if any("Cleaned" in t for t in result["texts"]):
        failures.append(f"unexpected Cleaned toast in: {result['texts']!r}")
    if any("Enriched" in t for t in result["texts"]):
        failures.append(f"unexpected Enriched toast in: {result['texts']!r}")
    if result["pickup"] is not None:
        failures.append(f"care success leaked into LastPickupMessage: {result['pickup']!r}")
    failures.extend(_assert_unrelated_unchanged(result["unrelated"]))
    return failures


def assert_clean(result: dict) -> list[str]:
    failures = []
    before, after = result["stat_before"], result["stat_after"]
    if before is None or after is None or after <= before:
        failures.append(f"cleanliness did not improve: {before} -> {after}")
    expected = f"Cleaned {result['animal']}"
    care_toast = next((t for t in result["texts"] if expected in t), None)
    if care_toast is None:
        failures.append(f"{expected!r} toast not found: {result['texts']!r}")
    if any("Enriched" in t for t in result["texts"]):
        failures.append(f"unexpected Enriched toast in: {result['texts']!r}")
    if result["pickup"] is not None:
        failures.append(f"care success leaked into LastPickupMessage: {result['pickup']!r}")
    failures.extend(_assert_unrelated_unchanged(result["unrelated"]))
    return failures


def assert_enrich(result: dict) -> list[str]:
    failures = []
    before, after = result["stat_before"], result["stat_after"]
    if before is None or after is None or after <= before:
        failures.append(f"happiness did not improve: {before} -> {after}")
    if result["consumed_item"] and result["consumed_item"] in str(result["slots"]):
        failures.append(f"enrich did not consume mirror: {result['slots']!r}")
    expected = f"Enriched {result['animal']}"
    care_toast = next((t for t in result["texts"] if expected in t), None)
    if care_toast is None:
        failures.append(f"{expected!r} toast not found: {result['texts']!r}")
    if any("Cleaned" in t for t in result["texts"]):
        failures.append(f"unexpected Cleaned toast in: {result['texts']!r}")
    if result["pickup"] is not None:
        failures.append(f"care success leaked into LastPickupMessage: {result['pickup']!r}")
    failures.extend(_assert_unrelated_unchanged(result["unrelated"]))
    return failures


FLOWS = {
    "feed": (run_player_feed_flow, assert_feed),
    "clean": (run_player_clean_flow, assert_clean),
    "enrich": (run_player_enrich_flow, assert_enrich),
}


def main(argv: list[str] | None = None) -> int:
    args = list(sys.argv[1:] if argv is None else argv)
    if len(args) != 1 or args[0] not in SCENARIOS:
        print(
            f"usage: {sys.argv[0]} {{{'|'.join(SCENARIOS)}}}",
            file=sys.stderr,
        )
        return 1
    scenario = args[0]

    if not wait_for_http():
        message = f"headless BRP not reachable at {BASE}"
        if ALLOW_MISSING_SERVER:
            print(f"skip: {message}", file=sys.stderr)
            return 0
        print(f"FAIL: {message}", file=sys.stderr)
        return 1

    run_flow, assert_flow = FLOWS[scenario]
    try:
        result = run_flow()
    except Exception as exc:  # noqa: BLE001 - driver boundary
        print(f"FAIL: {exc}", file=sys.stderr)
        return 1

    failures = assert_flow(result)
    if failures:
        print("FAIL:", file=sys.stderr)
        for failure in failures:
            print(f"  - {failure}", file=sys.stderr)
        return 1

    print(
        f"PASS: {scenario} {result['animal']} "
        f"({result['stat_before']} -> {result['stat_after']}); "
        f"tile={result['tile']}",
        flush=True,
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
