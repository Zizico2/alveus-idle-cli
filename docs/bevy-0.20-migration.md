# Bevy 0.20 migration

This platform work follows Roadmap Epic 15 (agent-native hardening). Gameplay
numbers continue to live in `alveus-configs`; this migration does not change balance.

## Implementation plan

1. Upgrade Bevy, its direct ECS/Reflect dependencies, enhanced input, Tiled,
   tweening, and Moonshine Save together, and regenerate the lockfile. Confirm
   the dependency graph contains a single Bevy version family.
2. Apply the [0.19 → 0.20 migration guide](https://bevy.org/learn/migration-guides/0-19-to-0-20/),
   including lifecycle observers, flat pointer events, sprite/rendering APIs,
   and reflection changes while preserving the command routing/state flush order.
3. Adopt BSN for reusable themed widgets and declarative screen/menu hierarchies.
   Use 0.20 scene inclusions (`@`), entity separators (`--`), and direct component
   expressions. Keep runtime-generated gameplay data in ordinary ECS systems.
4. Review the [0.20 improvements](https://bevy.org/news/bevy-0-20/) for relevant
   scheduling, UI, and scene improvements. Preserve strong ordering at deferred
   command/state boundaries; use new features where they solve an existing need.
5. Add regression coverage for scene hierarchies, focus/actions, and relevant
   protocol behavior. Run formatting, build/check, Clippy, documentation, default
   and headless tests, and the web check. Update CI to exercise the headless path.
6. Open a draft PR and resolve failures until all CI jobs pass.

## Adopted improvements

- Bevy/ECS/Reflect **0.20.1**, enhanced input **0.27.2**, Tiled **0.15.1**,
  tweening **0.17.0**, and Moonshine Save **0.8.0** resolve together. The lockfile
  contains no older Bevy engine family.
- Shared roots, labels, and buttons are composable BSN scenes. Small buttons
  patch layout fields on the shared button scene; action-list buttons and
  observer-backed buttons share the same implementation. Observer actions still
  trigger `GameCommand`.
- Splash, loading, fatal-error, settings, and credits use BSN hierarchies. All
  five enhanced-input contexts use BSN `Actions`/`Bindings` relationships and the
  new preset scene lists, preserving binding order and reset behavior.
- Settings use the new `Ready` event to initialize the volume label after the
  hierarchy exists, including when reopening without a volume change. Subsequent
  updates run only when the volume changes.
- Shared widget font sizes, dimensions, and spacing use `rem`. At Bevy's default
  `RemSize(20)` these retain their previous pixel sizes and now scale together
  when the root font size changes.
- Pointer observers use flat events and lifecycle observers carry their bundle
  type on the lifecycle event. Tiled accessors and export error handling follow
  the new APIs.
- CI tests both default and headless features. The compatible Bevy linter is
  pinned to an immutable upstream revision and required again.

The sprite backend, change tracking, and bulk despawn improvements arrive with
the engine upgrade. This project has no custom shader files to translate to
WESL. Strong schedule ordering remains at movement and command/state boundaries,
where deferred effects and gameplay sequencing are part of the contract.

## Reproducible rendered smoke

`python3 scripts/headless_bevy20_smoke.py` starts its own realtime BRP server,
uses a temporary save directory, checks settings initialization/reopening via
ECS text queries, and captures settings, credits, gameplay, and pause under
`screenshots/bevy20_*.png`. Each capture asserts composed UI pixels. The driver
always terminates its server. It requires a wgpu adapter; ordinary CI stays
GPU-independent. Unit and in-process BRP tests cover the settings scene flow.
