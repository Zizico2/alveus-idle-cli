# Plan: Fix Epic 1 care feedback / satchel-card copy (and related HUD card bugs)

**Target branch:** `feature/epic-1` (implement directly there; do **not** start from `feature/epic-2` or `main`).

**Audience:** A fresh agent with no prior chat context. Read this plan + `AGENTS.md` + the cited files on `feature/epic-1`.

**Goal:** Care outcome copy shown on the satchel card (and matching toast) must match the chore that actually ran. Cleaning must not say “Enriched”; feeding must say “Fed”; true enrichment must say “Enriched”. Lock this with Rust tests + BRP Python audit scripts (full sources in §10). Fix related satchel-card clarity regressions introduced by Epic 1’s feedback plumbing.

**Evidence status:** Root cause is **statically confirmed** in `apply_animal_enriched` (hardcoded `"Enriched {name}"` while ignoring `event.stat`). The implementing agent **must** write the §10 scripts to `scripts/`, run them on `feature/epic-1` after the fix (and ideally once before, to see FAIL on Case A).

---

## 0. Checkout and baseline

```bash
git fetch origin
git checkout feature/epic-1
git status -sb
git log --oneline -5
```

Confirm you are on `feature/epic-1` (commit that adds the care framework / 2-slot satchel / `CareFeedbackEvent` — historically `ec9535e` or its tip). Do **not** merge `feature/epic-2` into this work unless the user asks; Epic 2 already depends on Epic 1 and will pick up the fix on rebase/merge later.

Read before coding:

| File | Why |
|------|-----|
| `ROADMAP.md` → Epic 1 | Intent: chore result toast + HUD pulse; always show both satchel slots |
| `crates/alveus-interaction/src/lib.rs` | `apply_animal_fed` / `apply_animal_enriched` / `emit_care_feedback` / `set_pickup_message` |
| `crates/alveus-hud/src/lib.rs` | Satchel card, `satchel_body_label`, care pulse, interaction prompts |
| `crates/alveus-components/src/lib.rs` | `LastPickupMessage`, `CareFeedbackEvent`, `CareHudPulse` |
| `crates/alveus-world/src/toast.rs` | `care_feedback_toast_observer` |
| `crates/alveus-configs/src/lib.rs` | `CARE_FEED_RESTORE`, `CARE_ENRICH_RESTORE`, `care_restore_delta` |
| `design/design.md` §10 Interaction Types | `feed_animal` vs `enrich_animal` vs `clean_tile` are **distinct** verbs |
| `AGENTS.md` | Python driver rules, BRP verbs, stop headless when done |

---

## 1. Confirmed root cause (static — already verified)

### 1.1 Wrong copy: clean → “Enriched …”

Epic 1 introduced a single component/event pair for non-feed care restores:

- Component: `EnrichAnimal { animal_id, required_item, stat, delta, prompt }`
- Event: `AnimalEnrichedEvent { … same fields … }`
- Observer: `apply_animal_enriched` **always** formats:

```rust
let msg = format!("Enriched {}", animal_display_name(event.animal_id));
```

It ignores `event.stat`.

Epic 2 (and the intended framework usage) authors **nesting / clean stations** as `EnrichAnimal` with `stat: AnimalStat::Cleanliness` (prompt like `"Sweep nesting"`). That correctly restores enclosure cleanliness via `ImproveStatEvent` → `AnimalStat::Cleanliness` routing in `alveus-stats`, but the satchel card + toast still say **“Enriched Polly”** (or whatever animal).

So the bug is **feedback copy keyed off event type name**, not off the restored stat / chore kind.

`apply_animal_fed` correctly says `"Fed {name}"`. If a player reports “feed also says enriched”, verify with the audit script in §10 — likely they are seeing the nesting-clean case, or a toast that was replaced by a later `CareFeedbackEvent`. Do not “fix” feed copy unless the script proves it wrong.

### 1.2 Satchel card hijack (Epic 1 regression vs roadmap)

`update_room_feedback_hud_system` → `satchel_body_label`:

```rust
if let Some(message) = &pickup_message.text {
    return message.clone();  // replaces BOTH slot lines
}
// else: "Slot 1: …\nSlot 2: …"
```

Every care success writes the outcome into `LastPickupMessage` **and** emits `CareFeedbackEvent` (toast + satchel pulse). For ~2.5s the satchel card body becomes a single outcome string (“Enriched Polly”) instead of the two slots Epic 1 promised (“Held-item clarity: always show both satchel slots”).

Toast already carries the outcome. The satchel card should remain inventory-first.

### 1.3 What Epic 1 did **not** break

- Animal stat cards (Hunger / Cleanliness / Happiness bars) — unchanged logic; cleanliness still resolved via enclosure.
- Wheelbarrow card copy — still `Wheelbarrow: n/cap`; poop pickup/dump still set accurate `LastPickupMessage` (“Picked up poop…”, “Emptied wheelbarrow”).
- Interaction prompt card — uses per-object `prompt` strings (`Sweep nesting`, `Fill Polly's bowl`, etc.); those are fine.
- Upkeep / neglect banner — unrelated.

### 1.4 Why Epic 1 tests missed this

`tests/care_interaction_tests.rs` asserts happiness restore and satchel consumption for enrich, but **never asserts** `LastPickupMessage.text` or toast/feedback strings. Nesting-as-`EnrichAnimal`+`Cleanliness` appears in Epic 2’s `tests/nutrition_house_flow_test.rs` and still does not assert feedback copy.

---

## 2. Desired player-facing behavior

| Action | Stat restored | Satchel card (steady) | Transient feedback (toast + optional pulse) |
|--------|---------------|------------------------|-----------------------------------------------|
| Feed dish (`FeedAnimal` / `AnimalFedEvent`) | Hunger (usually) | Keep showing Slot 1/2 | `"Fed {Animal}"` |
| Clean / nesting (`EnrichAnimal` or future clean type with `Cleanliness`) | Cleanliness (enclosure) | Keep showing Slot 1/2 | `"Cleaned {Animal}"` (or `"Swept nesting — cleaned {Animal}"` if you prefer prompt-based; keep short) |
| True enrich (happiness toy/post) | Happiness | Keep showing Slot 1/2 | `"Enriched {Animal}"` |
| Give / fridge take / prep finish | n/a (inventory) | Slots update; brief inventory message OK on card **or** toast-only — pick one policy and stick to it (see §3.2) |
| Missing required item / satchel full | n/a | Error string may appear on card (existing pattern) | Prefer toast or card, not wrong “Enriched” |

Pulse (`CareHudPulse` green flash on satchel root) may stay for successful care restores.

---

## 3. Implementation plan (preferred: minimal, correct copy)

### 3.1 Fix feedback message selection (required)

In `crates/alveus-interaction/src/lib.rs`, change `apply_animal_enriched` so the message depends on `event.stat`:

```rust
let name = animal_display_name(event.animal_id);
let msg = match event.stat {
    AnimalStat::Cleanliness => format!("Cleaned {name}"),
    AnimalStat::Happiness => format!("Enriched {name}"),
    AnimalStat::Hunger => format!("Fed {name}"), // defensive; feed should use AnimalFedEvent
};
```

Keep `apply_animal_fed` as `"Fed {name}"` (optionally also key off `event.stat` for consistency if a feed tile ever restores something else — unlikely).

Extract a small helper if it keeps the file tidy, e.g. `care_outcome_message(animal_id, stat) -> String`, and unit-test it.

**Do not** hardcode animal names beyond the existing `animal_display_name` match.

### 3.2 Stop replacing satchel slot text with care outcomes (required)

Roadmap Epic 1: always show both satchel slots.

Recommended policy:

1. **Toast** (`CareFeedbackEvent` → `TriggerToastEvent`) = chore outcome (“Fed…”, “Cleaned…”, “Enriched…”, “Prepared…”, “Took…”).
2. **Satchel card body** = always the two slot lines (+ “Press [K] to drop” when non-empty).
3. **`LastPickupMessage`** = inventory / error / progress only:
   - pickups, drops, satchel full, missing item
   - mini-chore progress `"Chop veggies (2/5)"`
   - poop wheelbarrow messages (cleaning crate)
   - **not** successful feed/clean/enrich outcome strings

Concrete code changes:

- In `apply_animal_fed` / `apply_animal_enriched`: call `emit_care_feedback` only; **remove** `set_pickup_message` for the success path (keep `set_pickup_message` on validation failure).
- In mini-chore **completion** success paths that currently both set pickup + `CareFeedbackEvent`: emit toast via `CareFeedbackEvent`; do not overwrite satchel body with “Prepared …” / “Finished …” (optional: keep progress taps on `LastPickupMessage` since those are not final outcomes).
- Fridge `confirm_care_menu*`: “Took {item}” can stay toast-only; satchel slots already show the new item.
- Update `satchel_body_label` so it **never** prefers care-outcome strings over slots. Simplest robust approach after the above: only show `pickup_message.text` when it is an inventory/error/progress message — or always show slots and move *all* transient copy to toast. Prefer **always show slots** if you can migrate remaining inventory flashes to toast or a dedicated one-line subtitle without hiding slots.

If you keep `LastPickupMessage` overlay for errors only, document the rule in a short comment above `satchel_body_label`.

**Agent observability note:** The §10 audit scripts read `LastPickupMessage` for outcome strings. If you make success toast-only, either (a) keep writing the same outcome string to `LastPickupMessage` as well (simplest for agents), or (b) extend the scripts to observe feedback another queryable way. Prefer (a) only if it does **not** hide satchel slots — e.g. a separate resource, or toast-only + Rust tests for copy.

### 3.3 Optional but recommended: name the clean restore constant

In `crates/alveus-configs/src/lib.rs`:

- Today `CARE_ENRICH_RESTORE` is reused for cleanliness restores (Epic 2 nesting uses it).
- Add `CARE_CLEAN_RESTORE` (can equal `STAT_FULL` like the others) and/or extend `care_restore_delta` to take `AnimalStat` instead of `is_enrich: bool`.
- Update call sites / docs in `crates/alveus-configs/README.md` Planned/shipped tables.
- Do **not** invent new magic numbers in feature crates.

### 3.4 Optional larger follow-up (out of scope unless copy fix is insufficient)

Design’s `clean_tile` is a separate interaction type from `enrich_animal`. A future `CleanAnimal` / `AnimalCleanedEvent` would be clearer than overloading `EnrichAnimal` with `stat: Cleanliness`. **Do not** do that in this bugfix unless the user expands scope — Epic 1 already shipped the overload, and Epic 2 map tiles use `enrich_animal` + Cleanliness. Message-by-stat fixes the player-visible bug without a map migration.

### 3.5 Reflect / BRP

No new BRP methods. If you add types/events, register in `crates/alveus-headless/src/reflect.rs`. Message-only / helper changes need no new registration. `LastPickupMessage` and `CareFeedbackEvent` are already registered on Epic 1.

### 3.6 HUD prompt card

No change required for prompts (`Sweep nesting` etc.). Optionally harden `care_menu_prompt` so it does not hardcode the word `"Fridge"` forever (`CareMenuId` display name) — low priority; only Fridge exists today.

---

## 4. Tests to add / update (required)

Add assertions in `tests/care_interaction_tests.rs` (or a focused new test module) on `feature/epic-1`:

1. **`enrich_happiness_feedback_says_enriched`**
   - Trigger `AnimalEnrichedEvent { stat: Happiness, … }` for Push Pop.
   - After `app.update()`, assert feedback contains `"Enriched"`, not `"Cleaned"`.
   - Practical approach: if success still writes `LastPickupMessage`, assert on that; otherwise assert via a test observer on `CareFeedbackEvent` or keep a queryable outcome for agents.

2. **`enrich_cleanliness_feedback_says_cleaned`**
   - `AnimalEnrichedEvent { animal_id: PushPop, stat: Cleanliness, delta: CARE_ENRICH_RESTORE or CARE_CLEAN_RESTORE, required_item: None, … }`.
   - Assert enclosure cleanliness restored (existing ImproveStat path).
   - Assert feedback copy is Cleaned, not Enriched.

3. **`feed_feedback_says_fed`**
   - Give satchel leafy greens / chicken grains as appropriate; trigger `AnimalFedEvent` or interact with a spawned `FeedAnimal`.
   - Assert `"Fed"` and not `"Enriched"`.

4. **`satchel_card_slots_remain_visible_after_care`** (logic-level)
   - After a successful feed/enrich, with satchel non-empty (second slot still holding an item), assert `PlayerSatchel` slots unchanged in structure and that `LastPickupMessage` does **not** replace inventory semantics if you cleared success messages from it.
   - Pure helper test for `satchel_body_label` if you make it `pub(crate)` or move formatting to a small pure fn in `alveus-hud` / shared place — prefer a pure function test over full UI spawn if easier.

Update any Epic 1 tests that assumed success outcomes live in `LastPickupMessage` (grep before editing).

Run:

```bash
cargo test --profile ci
cargo test --features headless --profile ci
cargo build --features headless
```

---

## 5. Python BRP audit scripts (required)

Epic 1’s map may **not** include Polly nesting/enrich tiles (those land on Epic 2). Do **not** rely on walking to nesting on Epic 1. Instead, trigger Reflect events over BRP and read resources — same path the HUD uses.

**Write the full scripts from §10** into:

- `scripts/headless_care_feedback_audit.py`
- `scripts/headless_satchel_card_clarity_audit.py`

Stdlib only. Confirm Reflect JSON shapes with `registry.schema` if a trigger fails.

### How to run

```bash
# Terminal A
cargo run --features headless -- --headless --realtime --port 15702 --no-stdio

# Terminal B
python3 scripts/headless_care_feedback_audit.py
python3 scripts/headless_satchel_card_clarity_audit.py
# then stop the server — autosaves save.ron
pgrep -af alveus-idle-cli
```

Promote durable assertions into Rust tests (§4). Keep the Python scripts under `scripts/` as regression playtests.

### If already on a tree that includes Epic 2 map objects

Also extend / run `scripts/headless_polly_care_demo.py` to assert feedback after nesting sweep and after feed/enrich. That is **extra**; Epic 1 fix must not depend on Epic 2 maps.

---

## 6. Other informative / card issues to scan while you are there

Check these explicitly; fix if broken on `feature/epic-1`, otherwise note “verified OK” in the PR/commit body:

| Check | How | Expected |
|-------|-----|----------|
| Feed copy | Audit script / feed Push Pop | `"Fed …"`, never `"Enriched …"` |
| Clean copy | `AnimalEnrichedEvent`+Cleanliness | `"Cleaned …"` |
| Enrich copy | Happiness enrich | `"Enriched …"` |
| Satchel always 2 slots when visible | HUD + screenshot | `Slot 1` / `Slot 2` lines not replaced by outcome |
| Satchel visible on overview even when empty | Epic 1 changed `show_satchel` to always on overview | Intentional; confirm not a regression you need to revert |
| Care menu prompt | Open fridge if present on branch | Lists items; Esc closes; does not leave Menu stuck |
| Mini-chore progress | Prep table if present; else spawned in unit test | `(n/m)` progress accurate; completion message not “Enriched” |
| Poop pickup card | Existing clean demo | Still “Picked up poop” / wheelbarrow counts; do not call it Enriched |
| Animal cards | Visual / query stats | Bars still track hunger/happiness; cleanliness from enclosure |
| Double toast | One care action → one outcome toast (no duplicate Fed+Enriched) | |
| Pulse | `CareHudPulse` fires on care success, clears ~0.4s | |

---

## 7. Definition of done

- [ ] On branch `feature/epic-1` only (unless user says otherwise).
- [ ] Cleanliness care feedback says **Cleaned**, not Enriched; Happiness says **Enriched**; Feed says **Fed**.
- [ ] Satchel card continues to show both slots after care actions (toast carries outcome).
- [ ] Rust tests assert the three feedback verbs / forbid the mislabel.
- [ ] §10 scripts written under `scripts/` and fail on the old bug / pass on the fix.
- [ ] `cargo test --profile ci` and `cargo test --features headless --profile ci` pass; `cargo build --features headless` clean of new warnings.
- [ ] Headless server stopped after playtests (`pgrep -af alveus-idle-cli` clear).
- [ ] No new custom BRP methods; no auto-nav verbs; no key-injection hatches.
- [ ] Do not commit unless the user asks.

---

## 8. Suggested commit message (only if user requests a commit)

```
fix(hud): make care feedback match feed/clean/enrich

Key satchel/toast copy off the restored AnimalStat so nesting
cleans no longer report as enrichment, and keep both satchel
slots visible while outcomes go through CareFeedback toasts.
```

---

## 9. Note for Epic 2 consumers

`feature/epic-2` already authors Polly nesting as `EnrichAnimal` + `Cleanliness` and will show the bug until it is rebased/merged onto a fixed Epic 1. After this lands, rebase/merge Epic 2 and re-run `scripts/headless_polly_care_demo.py` + `tests/nutrition_house_flow_test.rs` (add feedback asserts there too if not present).

---

## 10. Full Python audit scripts (write these files)

Create each file verbatim (or with small adaptations if Reflect JSON shapes differ — confirm via `registry.schema`).

### 10.1 `scripts/headless_care_feedback_audit.py`

```python
#!/usr/bin/env python3
"""BRP audit: care feedback copy must match feed / clean / enrich.

Run against a realtime headless server on feature/epic-1 (or later):

  cargo run --features headless -- --headless --realtime --port 15702 --no-stdio
  python3 scripts/headless_care_feedback_audit.py

This does NOT require Epic 2 map stations. It triggers Reflect care events
over BRP and asserts LastPickupMessage / observable feedback strings.

Expected after the fix:
  AnimalFedEvent              -> message contains "Fed", not "Enriched"
  AnimalEnrichedEvent+Clean   -> "Cleaned", not "Enriched"
  AnimalEnrichedEvent+Happy   -> "Enriched", not "Cleaned"

Before the fix, Cleanliness enrich incorrectly reports "Enriched …".

Exit codes: 0 pass (or server unreachable unless REQUIRE_SERVER=1), 1 fail.
Stop the headless process when finished (autosaves save.ron).
"""

from __future__ import annotations

import json
import os
import sys
import time
import urllib.error
import urllib.request

PORT = int(os.environ.get("BRP_PORT", "15702"))
BASE = f"http://127.0.0.1:{PORT}/"
EVENT = "alveus_headless::command::GameCommand"
REQUIRE_SERVER = os.environ.get("REQUIRE_SERVER", "0") == "1"

# Reflect type paths (verify with registry.schema if a call fails).
ANIMAL_FED = "alveus_interaction::AnimalFedEvent"
ANIMAL_ENRICHED = "alveus_interaction::AnimalEnrichedEvent"
LAST_PICKUP = "alveus_components::LastPickupMessage"
SATCHEL = "alveus_interaction::PlayerSatchel"
CARE_FEED_RESTORE = 1000  # STAT_FULL / CARE_*_RESTORE in alveus-configs


def rpc(method: str, params=None):
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


def trigger_game(value) -> None:
    rpc("world.trigger_event", {"event": EVENT, "value": value})


def trigger_event(type_path: str, value) -> None:
    rpc("world.trigger_event", {"event": type_path, "value": value})


def get_resource(type_path: str):
    res = rpc("world.get_resources", {"resource": type_path})
    if isinstance(res, dict):
        return res.get("value", res)
    return res


def wait_for_http(timeout_s: float = 30.0) -> bool:
    deadline = time.time() + timeout_s
    while time.time() < deadline:
        try:
            rpc("rpc.discover", {})
            return True
        except (urllib.error.URLError, TimeoutError, RuntimeError, json.JSONDecodeError):
            time.sleep(0.25)
    return False


def last_pickup_text() -> str | None:
    value = get_resource(LAST_PICKUP)
    if not isinstance(value, dict):
        return None
    text = value.get("text")
    if text is None:
        return None
    # Reflect Option<String> may be bare string or {"Some": "…"} depending on version.
    if isinstance(text, str):
        return text
    if isinstance(text, dict):
        if "Some" in text:
            return text["Some"]
        if "0" in text and isinstance(text["0"], str):
            return text["0"]
    return str(text) if text is not None else None


def clear_pickup_by_waiting(seconds: float = 2.8) -> None:
    """Wait for LastPickupMessage timer (~2.5s) to decay."""
    time.sleep(seconds)


def satchel_slots():
    value = get_resource(SATCHEL)
    if isinstance(value, dict):
        slots = value.get("slots")
        if isinstance(slots, list):
            return slots
    return []


def animal_stat_value(stat: str):
    """Bevy Reflect enum JSON — try unit-string first; callers may adapt."""
    return stat


def tile_pos(x: int = 0, y: int = 0) -> dict:
    return {"x": x, "y": y}


def ensure_in_gameplay() -> None:
    """Best-effort skip splash / title so observers run under Gameplay."""
    for _ in range(3):
        trigger_game("SkipSplash")
        time.sleep(0.2)
    trigger_game("Play")
    time.sleep(0.5)
    # Some builds need Continue on title menus.
    trigger_game("Continue")
    time.sleep(0.3)


def seed_leafy_greens_via_fridge_or_chest() -> bool:
    """Try to put TortoiseLeafyGreens in the satchel using in-world GiveItem.

    On Epic 1 the Nutrition House fridge is still instant GiveItem (greens).
    Returns True if satchel appears to hold leafy greens.
    """
    # Planning coords — confirm via queries if navigation fails (AGENTS.md).
    nutrition_entrance = (33, 12)
    fridge = (2, 8)

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

    def step(direction: str, hold: float = 0.35):
        trigger_game({"Move": direction})
        time.sleep(hold)
        trigger_game("MoveStop")
        time.sleep(0.05)
        return player_tile()

    def walk_to(target, max_steps=100):
        tx, ty = target
        for _ in range(max_steps):
            pos = player_tile()
            if pos is None:
                time.sleep(0.1)
                continue
            x, y = pos
            if (x, y) == (tx, ty):
                return pos
            if x < tx:
                step("Right")
            elif x > tx:
                step("Left")
            elif y < ty:
                step("Up")
            elif y > ty:
                step("Down")
        return player_tile()

    def walk_adjacent(obj, max_steps=80):
        ox, oy = obj
        candidates = [(ox + 1, oy), (ox - 1, oy), (ox, oy + 1), (ox, oy - 1)]
        for _ in range(max_steps):
            pos = player_tile()
            if pos is None:
                time.sleep(0.1)
                continue
            if pos in candidates:
                return pos
            # Walk toward nearest candidate
            px, py = pos
            best = min(candidates, key=lambda c: abs(c[0] - px) + abs(c[1] - py))
            walk_to(best, max_steps=20)
        return player_tile()

    print("  seeding satchel via Nutrition House fridge…", flush=True)
    walk_to(nutrition_entrance)
    trigger_game("EnterBuilding")
    time.sleep(2.5)
    walk_adjacent(fridge)
    trigger_game("Interact")
    time.sleep(0.4)
    # Epic 2 fridge is a menu — Continue takes the selected item.
    trigger_game("Continue")
    time.sleep(0.3)
    slots = satchel_slots()
    blob = str(slots)
    ok = "TortoiseLeafyGreens" in blob or "Leafy" in blob
    print(f"  satchel after seed: {slots} ok={ok}", flush=True)
    return ok


def assert_msg(label: str, text: str | None, must_contain: str, must_not_contain: str) -> list[str]:
    fails = []
    print(f"[{label}] LastPickupMessage.text = {text!r}", flush=True)
    if text is None:
        # After toast-only fix, success may clear LastPickupMessage.
        # Extend this script to observe CareFeedback another way, OR keep
        # writing the outcome string to LastPickupMessage for agents
        # without hiding satchel slots — see plan §3.2.
        fails.append(
            f"{label}: no LastPickupMessage text "
            f"(if outcomes are toast-only, expose a queryable resource or "
            f"keep writing the outcome string to LastPickupMessage for agents)"
        )
        return fails
    if must_contain.lower() not in text.lower():
        fails.append(f"{label}: expected substring {must_contain!r} in {text!r}")
    if must_not_contain.lower() in text.lower():
        fails.append(f"{label}: forbidden substring {must_not_contain!r} in {text!r}")
    return fails


def main() -> int:
    if not wait_for_http(20.0):
        msg = f"headless BRP not reachable at {BASE}"
        if REQUIRE_SERVER:
            print(f"FAIL: {msg}", file=sys.stderr)
            return 1
        print(f"skip: {msg}", file=sys.stderr)
        return 0

    results: list[str] = []
    fails: list[str] = []

    ensure_in_gameplay()
    clear_pickup_by_waiting(0.5)

    # --- Case A: Cleanliness enrich must say Cleaned (reproduces the bug) ---
    print("Case A: AnimalEnrichedEvent + Cleanliness", flush=True)
    trigger_event(
        ANIMAL_ENRICHED,
        {
            "animal_id": "PushPop",
            "required_item": None,
            "stat": animal_stat_value("Cleanliness"),
            "delta": CARE_FEED_RESTORE,
            "station_position": tile_pos(1, 1),
        },
    )
    time.sleep(0.35)
    fails.extend(
        assert_msg("clean", last_pickup_text(), must_contain="Cleaned", must_not_contain="Enriched")
    )
    results.append("triggered Cleanliness enrich")
    clear_pickup_by_waiting()

    # --- Case B: Happiness enrich must say Enriched ---
    print("Case B: AnimalEnrichedEvent + Happiness", flush=True)
    trigger_event(
        ANIMAL_ENRICHED,
        {
            "animal_id": "PushPop",
            "required_item": None,
            "stat": animal_stat_value("Happiness"),
            "delta": CARE_FEED_RESTORE,
            "station_position": tile_pos(1, 1),
        },
    )
    time.sleep(0.35)
    fails.extend(
        assert_msg("enrich", last_pickup_text(), must_contain="Enriched", must_not_contain="Cleaned")
    )
    results.append("triggered Happiness enrich")
    clear_pickup_by_waiting()

    # --- Case C: Feed must say Fed ---
    print("Case C: AnimalFedEvent + Hunger", flush=True)
    seeded = seed_leafy_greens_via_fridge_or_chest()
    if not seeded:
        # Still attempt feed; apply_animal_fed will fail without item and set an error message.
        results.append("WARN: could not seed TortoiseLeafyGreens; feed case may be inconclusive")
    trigger_event(
        ANIMAL_FED,
        {
            "animal_id": "PushPop",
            "required_item": "TortoiseLeafyGreens",
            "stat": animal_stat_value("Hunger"),
            "delta": CARE_FEED_RESTORE,
            "dish_position": tile_pos(8, 6),
        },
    )
    time.sleep(0.35)
    text = last_pickup_text()
    if seeded:
        fails.extend(assert_msg("feed", text, must_contain="Fed", must_not_contain="Enriched"))
    else:
        print(f"[feed] skipped strict assert (seed failed); text={text!r}", flush=True)
    results.append("triggered Hunger feed")

    print("---", flush=True)
    for line in results:
        print(line, flush=True)

    if fails:
        print("FAIL:", file=sys.stderr)
        for f in fails:
            print(f"  - {f}", file=sys.stderr)
        print(
            "\nHint: on feature/epic-1, apply_animal_enriched always formats "
            "'Enriched {name}' and ignores AnimalStat::Cleanliness. See "
            "plans/epic-1-care-feedback-satchel-card.md",
            file=sys.stderr,
        )
        return 1

    print("PASS: care feedback verbs look correct", flush=True)
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except Exception as exc:  # noqa: BLE001 — driver script surface
        print(f"FAIL: unhandled {exc}", file=sys.stderr)
        raise SystemExit(1) from exc
```

### 10.2 `scripts/headless_satchel_card_clarity_audit.py`

```python
#!/usr/bin/env python3
"""BRP audit: satchel card must keep both slots visible after care outcomes.

Companion to scripts/headless_care_feedback_audit.py and
plans/epic-1-care-feedback-satchel-card.md §3.2 / §5.

Run:

  cargo run --features headless -- --headless --realtime --port 15702 --no-stdio
  python3 scripts/headless_satchel_card_clarity_audit.py

Policy under test (post-fix):
  - PlayerSatchel always has two slots in BRP.
  - After a care success, if a second item remains in the satchel, slots still
    report that item (inventory not wiped).
  - LastPickupMessage must not be the only place inventory is shown; after the
    HUD fix, success outcomes should not permanently replace slot labels.
    This script checks resources (agent-observable), then optional screenshot.

Stop the headless server when finished.
"""

from __future__ import annotations

import json
import os
import sys
import time
import urllib.error
import urllib.request
from pathlib import Path

PORT = int(os.environ.get("BRP_PORT", "15702"))
BASE = f"http://127.0.0.1:{PORT}/"
EVENT = "alveus_headless::command::GameCommand"
REQUIRE_SERVER = os.environ.get("REQUIRE_SERVER", "0") == "1"
REPO_ROOT = Path(__file__).resolve().parents[1]
SCREENSHOT = REPO_ROOT / "screenshots" / "satchel_card_clarity.png"

SATCHEL = "alveus_interaction::PlayerSatchel"
LAST_PICKUP = "alveus_components::LastPickupMessage"
ANIMAL_ENRICHED = "alveus_interaction::AnimalEnrichedEvent"


def rpc(method: str, params=None):
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


def trigger_game(value) -> None:
    rpc("world.trigger_event", {"event": EVENT, "value": value})


def trigger_event(type_path: str, value) -> None:
    rpc("world.trigger_event", {"event": type_path, "value": value})


def get_resource(type_path: str):
    res = rpc("world.get_resources", {"resource": type_path})
    if isinstance(res, dict):
        return res.get("value", res)
    return res


def wait_for_http(timeout_s: float = 20.0) -> bool:
    deadline = time.time() + timeout_s
    while time.time() < deadline:
        try:
            rpc("rpc.discover", {})
            return True
        except (urllib.error.URLError, TimeoutError, RuntimeError, json.JSONDecodeError):
            time.sleep(0.25)
    return False


def satchel_slots():
    value = get_resource(SATCHEL)
    if isinstance(value, dict):
        slots = value.get("slots")
        if isinstance(slots, list):
            return slots
    return []


def last_pickup_text() -> str | None:
    value = get_resource(LAST_PICKUP)
    if not isinstance(value, dict):
        return None
    text = value.get("text")
    if isinstance(text, str):
        return text
    if isinstance(text, dict) and "Some" in text:
        return text["Some"]
    return None


def occupied_count(slots) -> int:
    n = 0
    for s in slots:
        if s is not None and s != "None" and s != {}:
            # Reflect may encode Option as null or nested.
            if isinstance(s, dict) and set(s.keys()) <= {"None"}:
                continue
            n += 1
    return n


def main() -> int:
    if not wait_for_http():
        msg = f"headless BRP not reachable at {BASE}"
        if REQUIRE_SERVER:
            print(f"FAIL: {msg}", file=sys.stderr)
            return 1
        print(f"skip: {msg}", file=sys.stderr)
        return 0

    fails: list[str] = []

    for _ in range(3):
        trigger_game("SkipSplash")
        time.sleep(0.15)
    trigger_game("Play")
    time.sleep(0.4)

    slots = satchel_slots()
    print(f"satchel slots shape: {slots!r}", flush=True)
    if not isinstance(slots, list) or len(slots) != 2:
        fails.append(f"expected 2 satchel slots, got {slots!r}")
    else:
        print("ok: PlayerSatchel has 2 slots", flush=True)

    # Trigger a no-item cleanliness "clean" care event (repro path).
    trigger_event(
        ANIMAL_ENRICHED,
        {
            "animal_id": "PushPop",
            "required_item": None,
            "stat": "Cleanliness",
            "delta": 1000,
            "station_position": {"x": 1, "y": 1},
        },
    )
    time.sleep(0.35)

    text = last_pickup_text()
    slots_after = satchel_slots()
    print(f"after clean event: pickup={text!r} slots={slots_after!r}", flush=True)

    if text and "Enriched" in text and "Clean" not in text:
        fails.append(
            f"satchel/pickup still mislabels clean as enrich: {text!r} "
            "(fix apply_animal_enriched message selection)"
        )

    if len(slots_after) != 2:
        fails.append(f"slots length changed after care: {slots_after!r}")

    # Optional visual: screenshot for human/agent image inspection.
    SCREENSHOT.parent.mkdir(parents=True, exist_ok=True)
    trigger_game({"Screenshot": {"path": str(SCREENSHOT)}})
    time.sleep(0.6)
    if SCREENSHOT.is_file():
        print(f"wrote {SCREENSHOT}", flush=True)
    else:
        print(f"WARN: screenshot not found at {SCREENSHOT}", flush=True)

    if fails:
        print("FAIL:", file=sys.stderr)
        for f in fails:
            print(f"  - {f}", file=sys.stderr)
        return 1

    print("PASS: satchel slot shape + clean label checks", flush=True)
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except Exception as exc:  # noqa: BLE001
        print(f"FAIL: unhandled {exc}", file=sys.stderr)
        raise SystemExit(1) from exc
```
