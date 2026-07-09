# Concept notes (lightweight)

Successor to the old JSON-schema design pipeline. Use this folder for **optional** deeper intent when an epic brief in [`ROADMAP.md`](../../ROADMAP.md) is not enough.

## Process

1. Pick an epic section from `ROADMAP.md` (respect **Depends on**).
2. Optionally add `docs/concepts/<feature>.md` for narrative / UX intent only.
3. Put concrete numbers in [`alveus-configs`](../../crates/alveus-configs):
   - Already shipping → Rust (`src/lib.rs`)
   - Not shipping yet → Planned ballparks in that crate’s `README.md`, then **promote** into Rust when implementing
4. Implement + tests. Prefer epic → agent plan → code + tests.
5. Do **not** require regenerating room JSON or schemas (removed in Epic 0).

## Lore

Siren = Blue-fronted Amazon memorial (see ROADMAP). Snake shed → future snake ambassadors.
