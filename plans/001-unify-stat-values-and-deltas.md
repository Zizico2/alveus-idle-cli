# Plan 001: Unify stored stat values, care deltas, and decay deltas behind `Stat`

> **Executor instructions**: Follow this plan step by step. Run every verification
> command and confirm the expected result before moving on. Read `AGENTS.md` and
> the repository `README.md` before editing. If anything in “STOP conditions”
> occurs, stop and report it; do not improvise. When finished, update Plan 001's
> status in `plans/README.md` unless a reviewer says they maintain the index.
>
> **Drift check (run first)**:
> `git diff --stat dbadfc4..HEAD -- crates/alveus-types crates/alveus-configs crates/alveus-stats crates/alveus-interaction crates/alveus-cleaning crates/alveus-hud crates/alveus-content crates/alveus-headless assets/maps tools tests scripts`
>
> If any in-scope file changed since this plan was written, compare the “Current
> state” excerpts and symbol inventory with live code before proceeding. A
> semantic mismatch is a STOP condition.

## Status

- **Priority**: P1
- **Effort**: L (multi-crate type migration, asset schema migration, save/BRP compatibility tests)
- **Risk**: HIGH (persistent save shape and reflected BRP/Tiled wire shapes are compatibility boundaries)
- **Depends on**: none
- **Category**: tech-debt / type-safety migration
- **Planned at**: commit `dbadfc4`, 2026-07-11

## Decision

Adopt one agnostic stat-unit newtype:

```rust
pub struct Stat(pub u32);
```

Use `Stat` for both current stored values and discrete changes, regardless of
direction. Direction remains in the operation (`ImproveStatEvent` versus
`WorsenStatEvent`), not in the value type. Rename the action-specific wrappers
and make them contain `Stat`:

```rust
pub struct FeedStat(pub Stat);
pub struct EnrichStat(pub Stat);
pub struct CleanStat(pub Stat);
```

The resulting vocabulary is:

| Concept | Type after migration |
|---|---|
| Stored hunger, happiness, cleanliness | `Stat` |
| Scale maximum and full initial value | `Stat` |
| Generic improve amount | `Stat` |
| Generic worsen/decay amount | `Stat` |
| Feed-authored delta | `FeedStat` wrapping `Stat` |
| Enrichment-authored delta | `EnrichStat` wrapping `Stat` |
| Cleaning-authored delta | `CleanStat` wrapping `Stat` |
| Continuous rate / fractional carry | `f32` (unchanged) |
| Stat axis selector | `AnimalStat` / `EnclosureStat` (unchanged) |

This is intentionally not an enum. The axis, action, and direction are already
represented by `StatTarget`, care component/event types, and improve/worsen event
types. An enum inside the numeric value would duplicate those dimensions and
permit contradictory combinations.

## Why this matters

The current implementation types positive deltas as `Restore` but leaves stored
stats and negative deltas as `u32`. That creates an artificial distinction:
`Restore(250)` and `250` in a decay event are the same unit on the same
`0..=STAT_SCALE` scale. It also allows unrelated `u32` values—tile sizes, counts,
timestamps, capacities—to be passed into worsening and stat math without an
explicit boundary.

The migration should make the stat unit consistent from configuration, through
stored ECS state and care/decay events, to HUD and cleaning math. Action wrappers
retain the useful guarantee that feeding, enrichment, and cleaning deltas cannot
be accidentally interchanged. Floating-point rates and accumulators remain
`f32`, because they represent “stat units per hour” and fractional carry rather
than a discrete stat value.

## Current state and evidence

- `crates/alveus-types/src/restore.rs:3-44` defines `Restore`, `FeedRestore`,
  `EnrichRestore`, and `CleanRestore`, all directly wrapping `u32`; conversions
  exist only from each action wrapper to positive `Restore` and from `Restore`
  to `u32`.
- `crates/alveus-configs/src/lib.rs:20-48` leaves `STAT_SCALE` and `STAT_FULL` as
  `u32`, while care constants use restore-specific wrappers.
- `crates/alveus-stats/src/lib.rs:38-42,87-90` stores hunger, happiness, and
  cleanliness as bare `u32`.
- `crates/alveus-stats/src/lib.rs:133-145` gives `ImproveStatEvent` a `Restore`
  amount but `WorsenStatEvent` a `u32` amount.
- `crates/alveus-stats/src/lib.rs:269-518` converts improve amounts to `u32` and
  runs both improvement and worsening arithmetic on bare integers.
- `crates/alveus-configs/src/lib.rs:310-425` uses a mix of `CleanRestore`, `u32`
  thresholds, `u32` current cleanliness, `u32` outputs, and `f32` rates in one
  cleaning domain.
- `crates/alveus-stats/src/lib.rs:803-839,916-959,1143-1178` creates worsening
  amounts from offline decay, real-time accumulators, and time-skip helpers as
  bare `u32`.
- `crates/alveus-headless/src/command.rs:101-108` exposes positive debug amounts
  as `Restore` and negative debug amounts as `u32` over BRP.
- `crates/alveus-headless/src/reflect.rs:28,74-79` registers the four old types.
- `crates/alveus-hud/src/lib.rs:549-580` resolves stat values as `Option<u32>` and
  performs percentage math with `STAT_SCALE as f32`.
- `assets/maps/interiors/interiors.tsx:121-125` currently authors a
  `FeedRestore` class whose tuple member `0` is an integer. Nesting `Stat` inside
  `FeedStat` may change this class shape and must be verified from generated
  Tiled metadata rather than guessed.
- `tests/brp_tests.rs:207-282` establishes that Bevy Reflect currently accepts
  the `Restore` newtype as a bare number on the BRP wire and rejects `{ "0": 250
  }`. The replacement must preserve or deliberately characterize this behavior.
- `save.ron:20-100` persists `AnimalStats` and `EnclosureStats` fields as scalar
  integers. Existing saves must continue to load, or the executor must stop and
  report the incompatibility before changing the format.

The old plan `plans/epic-1-care-restore-newtypes.md` is historical context only.
Its explicit choice to keep stored values and worsening as `u32` is superseded
by this plan.

## Target type API

Create `crates/alveus-types/src/stat.rs` and remove the old `restore.rs` after
all callers migrate. The target shape is:

```rust
use bevy_reflect::Reflect;

/// Discrete units on the game's stat scale.
///
/// This type is direction-agnostic: it can be a stored value, an improvement
/// amount, or a worsening amount. Callers that mutate stored state remain
/// responsible for clamping to the configured scale.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Reflect)]
#[type_path = "alveus_types"]
pub struct Stat(pub u32);

impl Stat {
    pub const ZERO: Self = Self(0);

    pub const fn get(self) -> u32 { self.0 }
    pub const fn is_zero(self) -> bool { self.0 == 0 }

    pub const fn saturating_add_capped(self, amount: Self, cap: Self) -> Self {
        Self(self.0.saturating_add(amount.0).min(cap.0))
    }

    pub const fn saturating_sub(self, amount: Self) -> Self {
        Self(self.0.saturating_sub(amount.0))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Reflect)]
#[type_path = "alveus_types"]
pub struct FeedStat(pub Stat);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Reflect)]
#[type_path = "alveus_types"]
pub struct EnrichStat(pub Stat);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Reflect)]
#[type_path = "alveus_types"]
pub struct CleanStat(pub Stat);
```

Implement `From<FeedStat> for Stat`, `From<EnrichStat> for Stat`, and
`From<CleanStat> for Stat`. An `impl From<Stat> for u32` is acceptable for
ergonomic logging/FFI boundaries, but gameplay code should prefer `.get()` at a
deliberate numeric boundary. Do **not** implement:

- `From<u32> for Stat` or `From<u32>` for an action wrapper; construction from a
  primitive must stay visible as `Stat(n)`.
- arithmetic traits between `Stat` and primitives;
- cross-action conversions (`FeedStat` to `CleanStat`, etc.);
- `Deref<Target = u32>`;
- an internal clamp to 1000 in `Stat` itself. The foundational `alveus-types`
  crate must not depend on `alveus-configs`, and a delta may legitimately exceed
  the cap before saturating application.

The tuple field stays public to match repository identifier/newtype style and
Reflect/Tiled needs. Centralize mutating arithmetic in the methods above so cap
and saturation policy is reviewable.

## Commands you will need

| Purpose | Command | Expected on success |
|---|---|---|
| Inspect drift | `git diff --stat dbadfc4..HEAD -- <in-scope paths>` | no unexplained semantic drift |
| Find old names | `rg -n '\b(Restore|FeedRestore|EnrichRestore|CleanRestore)\b' crates tests src tools assets scripts` | no matches after migration, except intentional historical plan text |
| Find bare stat fields/events | `rg -n 'pub (hunger|happiness|cleanliness): u32|WorsenStat \{ target: StatTarget, amount: u32|pub amount: u32' crates/alveus-stats crates/alveus-headless` | no stat-domain matches |
| Generate Tiled schema | `cargo run --bin gen_tiled_types` | exit 0; `tiled_types.json` contains `Stat`, `FeedStat`, `EnrichStat`, `CleanStat` |
| Focused type tests | `cargo test -p alveus-types --profile ci` | all pass |
| Default tests | `cargo test --profile ci` | all pass |
| Headless/BRP tests | `cargo test --features headless --profile ci` | all pass |
| Headless build | `cargo build --features headless` | exit 0; no new warnings |
| Workspace lint | `cargo clippy --workspace -- -D warnings` | exit 0; no warnings |

Do not run a live headless server for this migration unless a test cannot cover a
BRP observation. If one is started, drive it through one stdlib Python script in
`scripts/`, stop it afterward, and verify `pgrep -af alveus-idle-cli` has no
server process, per `AGENTS.md`.

## Scope

**In scope** (modify only where required by the type migration):

- `crates/alveus-types/src/stat.rs` (create)
- `crates/alveus-types/src/restore.rs` (delete after migration)
- `crates/alveus-types/src/lib.rs`
- `crates/alveus-configs/src/lib.rs`
- `crates/alveus-configs/README.md`
- `crates/alveus-stats/src/lib.rs`
- `crates/alveus-interaction/src/lib.rs`
- `crates/alveus-cleaning/src/lib.rs`
- `crates/alveus-hud/src/lib.rs`
- `crates/alveus-content/src/lib.rs` if its re-exports or scale calculations require it
- `crates/alveus-headless/src/command.rs`
- `crates/alveus-headless/src/reflect.rs`
- `assets/maps/interiors/interiors.tsx`
- `assets/maps/overview/tiled_types.json` (generated output)
- `tools/gen_interiors.py`
- existing Rust tests under `tests/` and `crates/*` that construct or assert stat values
- one focused compatibility fixture/test file if needed
- `scripts/` only if a retained BRP compatibility script is genuinely required
- `plans/README.md` status only

**Out of scope**:

- Renaming `AnimalStat`, `EnclosureStat`, `StatTarget`, `ImproveStatEvent`, or
  `WorsenStatEvent`.
- Combining improve/worsen events into an enum.
- Rebalancing any numeric gameplay value.
- Changing normalized decay-rate configuration from `f32`.
- Replacing fractional decay accumulators with fixed-point math.
- Adding new care actions, rooms, animals, or BRP methods.
- Editing historical prose under `design/` or regenerating design JSON.
- Opportunistic HUD, cleaning, save-system, or interaction refactors.
- Editing `save.ron` in place as a migration strategy.

## Git workflow

- Work on the current feature branch unless the operator requests a new branch.
  If creating one, use `codex/unify-stat-type`.
- The recent repository history uses short imperative/descriptive messages such
  as `newtypes` and `fixing reviews`; use a concise message such as
  `unify stat values and deltas`.
- Do not push or open a pull request unless explicitly instructed.
- Preserve unrelated working-tree changes. Check `git status --short` before
  editing and before handoff.

## Steps

### Step 1: Establish save and Reflect compatibility before renaming

Before changing production types, add characterization tests for the two risky
serialization boundaries:

1. A focused `Stat` newtype probe (temporarily local to a test if necessary)
   confirming how a one-field tuple newtype wrapping `u32` serializes through
   Bevy Reflect/BRP: expected input is a bare JSON number.
2. A nested action wrapper probe confirming the accepted JSON/Tiled shape of
   `FeedStat(Stat(1000))`. Do not assume whether Reflect flattens both newtypes.
3. A save compatibility test using an isolated fixture with the current scalar
   shape, e.g. `hunger: 495`, `happiness: 500`, `cleanliness: 495`. Load it
   through the same Moonshine Save path used by `StatsPlugin` and assert exact
   values after the target `Stat` fields are introduced.

Do not use the mutable repository-root `save.ron` as the fixture. Copy only the
minimal relevant RON structure into a test fixture or generate a temporary file
in the test, and always clean it up.

**Verify before production edits**: run the narrow characterization test. Record
the observed wire shapes in test names/comments. Expected result: exit 0 and a
known representation for `Stat`, nested care wrappers, and old scalar save data.

### Step 2: Replace restore-specific types with the agnostic hierarchy

Create `crates/alveus-types/src/stat.rs` using the target API above. Add small
unit tests for:

- `Stat(900).saturating_add_capped(Stat(200), Stat(1000)) == Stat(1000)`;
- addition below the cap;
- subtraction below zero produces `Stat::ZERO`;
- subtraction in range;
- all three action wrappers convert to the same inner `Stat` exactly;
- `get()` and `is_zero()`.

Export `Stat`, `FeedStat`, `EnrichStat`, and `CleanStat` from
`crates/alveus-types/src/lib.rs`. Do not delete `restore.rs` until all production
and test callers have migrated, so intermediate compiler errors remain localized.

**Verify**: `cargo test -p alveus-types --profile ci` → all tests pass.

### Step 3: Retype the canonical configuration source

In `crates/alveus-configs/src/lib.rs`:

```rust
pub const STAT_SCALE: Stat = Stat(1000);
pub const STAT_FULL: Stat = STAT_SCALE;

pub const CARE_FEED_RESTORE: FeedStat = FeedStat(STAT_FULL);
pub const CARE_ENRICH_RESTORE: EnrichStat = EnrichStat(STAT_FULL);
pub const CARE_CLEAN_RESTORE: CleanStat = CleanStat(STAT_FULL);
```

Keep the constant names `CARE_*_RESTORE` unless the maintainer explicitly wants
a second naming change. They describe the gameplay effect (“restore this many
stat units”), while the types now describe the direction-agnostic unit. The
requested semantic type renames are `FeedRestore` → `FeedStat`, `EnrichRestore`
→ `EnrichStat`, `CleanRestore` → `CleanStat`, and `Restore` → `Stat`.

Retype other discrete stat-scale configuration:

- `PoopConfig.spawn_thresholds: &'static [Stat]`;
- `PoopConfig.cleanliness_restore_per_poop: CleanStat`;
- construct thresholds as `Stat(800)`, `Stat(500)`, `Stat(200)`;
- keep `poop_decay_rate`, normalized animal decay rates, and enclosure
  `cleanliness_decay_per_hour` as `f32`;
- replace primitive casts like `STAT_SCALE as f32` with
  `STAT_SCALE.get() as f32` only at numeric/rate boundaries.

Retype cleaning functions according to meaning:

```rust
pub fn target_poop_count(cleanliness: Stat, thresholds: &[Stat]) -> u32;
pub fn cleanliness_after_threshold_decay(start: Stat, ...) -> Stat;
pub fn enclosure_cleanliness_decay_amount(start: Stat, ...) -> Stat;
```

Counts remain `u32`/`usize`; hours and rates remain `f32`. Within threshold math,
extract primitives only where floating-point division/multiplication requires
it, and wrap rounded decay back into `Stat` immediately. Prefer `Stat` methods
for subtraction and comparisons.

Update `crates/alveus-configs/README.md` to describe `Stat` as the shared unit,
the three action wrappers, typed scale/full constants, typed thresholds, and the
fact that rates/accumulators remain `f32`.

**Verify**: `cargo check -p alveus-configs` → exit 0.

### Step 4: Retype stored ECS state and both mutation events

In `crates/alveus-stats/src/lib.rs` change:

```rust
pub struct AnimalStats {
    pub hunger: Stat,
    pub happiness: Stat,
}

pub struct EnclosureStats {
    pub cleanliness: Stat,
}

pub struct ImproveStatEvent {
    pub target: StatTarget,
    pub amount: Stat,
}

pub struct WorsenStatEvent {
    pub target: StatTarget,
    pub amount: Stat,
}
```

Keep `AnimalDecayRates`, `EnclosureDecayRates`, and both accumulator components
as `f32`. Update all spawn/default paths to use `STAT_FULL` directly.

In both observers, stop converting the event amount to a primitive. Apply:

```rust
value.saturating_add_capped(event.amount, STAT_SCALE)
value.saturating_sub(event.amount)
```

Use `.get()` only for log formatting if `Display` is not implemented. Do not add
`Display` solely to avoid a few explicit boundaries unless it materially
improves diagnostics.

Update upkeep aggregation so totals cannot accidentally infer `Stat` arithmetic.
Use a deliberately wide primitive accumulator (`u64`) and add
`stats.hunger.get() as u64`, etc. Divide by `STAT_SCALE.get() as f32`. This also
avoids overflow if the animal roster grows.

**Verify**: `cargo check -p alveus-stats` → exit 0. Expected compile errors after
this step must be only downstream callers not yet migrated; errors inside
`alveus-stats` are not acceptable.

### Step 5: Retype every decay production boundary

Audit every `WorsenStatEvent` constructor and every helper returning a discrete
decay. Required paths include:

- offline elapsed-time decay in `apply_offline_decay_system`;
- real-time `tick_decay_system` after an accumulator crosses one unit;
- simulated time-skip helpers (`trigger_animal_decay`,
  `trigger_enclosure_decay`, and their query/world callers);
- debug keyboard worsening;
- headless `GameCommand::WorsenStat` dispatch;
- test fixtures and direct event triggers.

The rule is: rate math may be primitive/floating-point, but the rounded/floored
whole-unit result becomes `Stat(...)` at the point it enters the discrete stat
pipeline. Change `trigger_enclosure_decay(..., amount: Stat)` and use
`amount.is_zero()`.

Do not wrap unrelated `u32` values such as offline wander steps, timestamps,
poop counts, wheelbarrow counts, or frame counts.

**Verify**:

```bash
rg -n 'WorsenStatEvent \{' crates tests
rg -n 'amount: [a-zA-Z_][a-zA-Z0-9_]* as u32|amount: [0-9]+' crates/alveus-stats crates/alveus-headless
```

Inspect every match. Expected result: all worsening amounts are `Stat` values or
variables statically typed as `Stat`; no bare numeric event amounts remain.

### Step 6: Migrate care, cleaning, and HUD consumers

In `crates/alveus-interaction/src/lib.rs`, rename imports and all component/event
fields:

- `FeedAnimal.delta` and `AnimalFedEvent.delta`: `FeedStat`;
- `EnrichAnimal.delta` and `AnimalEnrichedEvent.delta`: `EnrichStat`;
- `CleanAnimal.delta` and `AnimalCleanedEvent.delta`: `CleanStat`.

Their observers continue using `event.delta.into()` to produce the generic
`Stat`. Preserve the existing fixed action-to-axis mapping and feedback copy.

In `crates/alveus-cleaning/src/lib.rs`, pass stored `Stat` cleanliness directly
to config math. Convert `CleanStat` to `Stat` for `ImproveStatEvent`. Keep poop
and wheelbarrow counts primitive.

In `crates/alveus-hud/src/lib.rs`, make `resolve_stat` return `Option<Stat>`.
Compute percentages with `val.get()` and `STAT_SCALE.get()`. Do not change
rounding, labels, colors, or layout.

In `crates/alveus-content/src/lib.rs`, keep any public re-export behavior but
adapt uses of typed `STAT_SCALE`/`STAT_FULL` via `.get()` only where consumers
require primitives.

**Verify**: `cargo check --workspace` → exit 0.

### Step 7: Preserve the BRP observation and command contracts

In `crates/alveus-headless/src/command.rs`, change both debug verbs to the same
quantity type:

```rust
ImproveStat { target: StatTarget, amount: Stat },
WorsenStat { target: StatTarget, amount: Stat },
```

Update both doc comments to say `amount` is a direction-agnostic stat-scale
quantity and document its **observed** Reflect wire representation. Do not add a
wire-only DTO or custom BRP method.

In `crates/alveus-headless/src/reflect.rs`, register `Stat`, `FeedStat`,
`EnrichStat`, and `CleanStat`; remove registrations for all four old types.
Because `AnimalStats` and `EnclosureStats` are queryable, verify that
`world.query` still returns scalar numeric field values (or record the actual
new reflected shape in tests if Bevy necessarily changes it).

Update `tests/brp_tests.rs` with symmetric tests:

- `ImproveStat` accepts the observed scalar `Stat` amount and changes the value;
- `WorsenStat` accepts the same shape and changes the value;
- the known-wrong tuple-map shape is rejected for both, if scalar remains the
  accepted representation;
- `world.query` for `AnimalStats` and `EnclosureStats` asserts the exact JSON
  shape external agents will receive;
- `registry.schema` exposes all four new types and none of the old names.

Update `tests/reflect_registry_tests.rs` accordingly.

**Verify**: run the focused BRP and registry tests with headless features; all
pass and explicitly lock the external wire shape.

### Step 8: Regenerate and migrate Tiled authoring

Change `tools/gen_interiors.py` helper and all generated/hand-authored map
properties from the old type paths to:

- `alveus_types::FeedStat`;
- `alveus_types::EnrichStat`;
- `alveus_types::CleanStat`.

Run `cargo run --bin gen_tiled_types` before deciding the XML member shape. A
wrapper around `Stat` may produce a nested class member, conceptually:

```xml
<property name="delta" type="class" propertytype="alveus_types::FeedStat">
  <properties>
    <property name="0" type="class" propertytype="alveus_types::Stat">
      <properties>
        <property name="0" type="int" value="1000"/>
      </properties>
    </property>
  </properties>
</property>
```

That is only an expectation. The generated `tiled_types.json` is authoritative.
Match its actual class/member layout exactly. Do not hand-flatten the property or
write custom reflection code merely to preserve the old XML.

Rename and expand `tests/tiled_feed_restore_test.rs` (for example,
`tests/tiled_care_stat_test.rs`) so it registers `Stat` plus relevant wrappers,
loads the real interior asset, and asserts exact hydration such as
`FeedStat(STAT_FULL)`. If current assets include enrich/clean authored objects,
test those too; otherwise do not invent new gameplay content merely for this
test—use a minimal in-memory fixture if all three shapes need coverage.

**Verify**:

```bash
cargo run --bin gen_tiled_types
rg -n '\b(Restore|FeedRestore|EnrichRestore|CleanRestore)\b' assets tools
cargo test --profile ci --test tiled_care_stat_test
```

Expected: generation exits 0; old type paths have no matches; the real map
hydrates the new nested stat type exactly.

### Step 9: Migrate tests without erasing type-safety

Update all test construction and assertions. Prefer explicit values:

```rust
stats.hunger = Stat(500);
assert_eq!(stats.hunger, Stat(750));
amount: Stat(300),
assert_eq!(feed.delta, FeedStat(STAT_FULL));
```

Do not make `Stat` comparable to `u32` merely to reduce test edits. The compile
friction is the point: it identifies every stat-domain boundary.

Required regression coverage:

- improve caps at `STAT_SCALE`;
- worsen saturates at `Stat::ZERO`;
- improve and worsen accept the same `Stat` amount type;
- real-time, offline, and time-skip decay produce identical values to the
  pre-migration behavior;
- poop thresholds and threshold-crossing decay preserve existing results;
- feed/enrich/clean only mutate their designated axes;
- upkeep percentages are unchanged;
- old scalar save fixture loads exactly;
- BRP and Tiled reflected shapes are locked.

**Verify**: `cargo test --profile ci` → all pass.

### Step 10: Remove old vocabulary and perform full verification

Delete `crates/alveus-types/src/restore.rs`, remove its module declaration, and
remove all old imports/registrations. Update comments/docstrings and
`crates/alveus-configs/README.md`; do not rewrite historical plan files.

Run:

```bash
rg -n '\b(Restore|FeedRestore|EnrichRestore|CleanRestore)\b' crates tests src tools assets scripts
cargo fmt --all -- --check
cargo test --profile ci
cargo test --features headless --profile ci
cargo build --features headless
cargo clippy --workspace -- -D warnings
git status --short
```

Expected results:

- no old type-name matches outside historical `plans/` documents;
- formatting check exits 0 (run `cargo fmt --all` once if needed, then re-check);
- both test suites, build, and clippy exit 0;
- generated Tiled metadata is included if changed;
- only in-scope files plus `plans/README.md` are modified.

## Test plan summary

Use existing tests as structural exemplars:

- `tests/stats_tests.rs` for saturation, axis routing, decay, and upkeep;
- `tests/cleaning_tests.rs` for typed thresholds and segmented decay;
- `tests/care_interaction_tests.rs` for action-wrapper routing;
- `tests/brp_tests.rs` for exact Reflect command shapes;
- `tests/reflect_registry_tests.rs` for registration;
- `tests/tiled_feed_restore_test.rs` for real asset hydration;
- `tests/play_crash_test.rs` for save/load behavior, supplemented by a focused
  scalar-save compatibility fixture.

Every pre-existing numeric expectation must stay numerically identical. The
migration changes type semantics, not balance or gameplay behavior.

## Done criteria

- [ ] `Stat`, `FeedStat`, `EnrichStat`, and `CleanStat` are the only stat-unit
      newtypes exported by `alveus-types`.
- [ ] `STAT_SCALE` and `STAT_FULL` are `Stat` constants.
- [ ] Stored hunger, happiness, and cleanliness fields are `Stat`.
- [ ] Both `ImproveStatEvent.amount` and `WorsenStatEvent.amount` are `Stat`.
- [ ] `GameCommand::ImproveStat` and `GameCommand::WorsenStat` use `Stat` and
      their doc comments state the verified BRP shape.
- [ ] Feed/enrich/clean component and event deltas use their corresponding
      action wrapper around `Stat`.
- [ ] Discrete cleaning thresholds and decay results use `Stat`; counts remain
      primitive and rates/accumulators remain `f32`.
- [ ] Existing scalar save data loads without losing or resetting values.
- [ ] BRP `world.trigger_event`, `world.query`, and `registry.schema` shapes are
      covered by exact protocol-level tests.
- [ ] Real Tiled assets hydrate the new wrapper hierarchy in a test.
- [ ] No gameplay number changed.
- [ ] No old type-name matches remain outside historical plan documents.
- [ ] Default and headless test suites, headless build, and workspace clippy pass.
- [ ] No headless server remains running.
- [ ] No files outside scope are modified, aside from executor status in
      `plans/README.md`.

## STOP conditions

Stop and report; do not improvise if:

- An existing scalar `save.ron` shape cannot deserialize into `Stat` fields via
  the current Moonshine/Reflect path. Report the exact error and a minimal
  reproduction; a save versioning/migration design requires maintainer approval.
- Bevy Reflect requires different BRP representations for improve and worsen
  despite both using `Stat`.
- Nested `FeedStat(Stat(...))` cannot be represented or hydrated by
  `bevy_ecs_tiled` using generated class metadata. Do not add custom BRP methods,
  bespoke observation structs, or speculative manual Reflect implementations.
- The generated Tiled schema contradicts the proposed wrapper hierarchy in a
  way that requires flattening or changing public types.
- Any numeric behavior changes in decay, thresholds, upkeep, or care restores.
- The migration appears to require changing `f32` rates/accumulators or unrelated
  count/timestamp/frame types.
- In-scope code has semantically drifted since `dbadfc4` and no longer matches
  this plan.
- A verification command fails twice after one reasonable correction.
- Unrelated working-tree changes overlap an in-scope file and cannot be safely
  preserved.

## Maintenance notes

- `Stat` deliberately does not know the configured maximum. Continue using
  `STAT_SCALE` at mutation boundaries; do not silently clamp construction.
- Future discrete stat values/deltas should use `Stat`. Future fractional rates
  or fractional carry should use an explicitly named rate/accumulator type or
  `f32`, not `Stat`.
- Future care actions should get an action-specific wrapper around `Stat` only
  when preventing cross-action mixing has value. Generic debug/system deltas use
  `Stat` directly.
- Reviewers should scrutinize save compatibility, exact BRP JSON, generated
  Tiled shape, and accidental primitive conversions more than mechanical rename
  noise.
- A later cleanup may rename `CARE_*_RESTORE` constants to `CARE_*_STAT`, but that
  is a separate vocabulary decision. This plan keeps their effect-oriented names
  to limit churn while adopting the requested type semantics.
