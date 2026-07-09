# Dev recipes for alveus-idle-cli.
# Requires: https://github.com/casey/just
# Cargo parallelism is set in `.cargo/config.toml` (`[build] jobs`).

# Regenerate tiled_types.json, then run the game (preferred local loop).
dev-run *args:
    mkdir -p screenshots
    cargo run --bin gen_tiled_types
    cargo run --bin alveus-idle-cli -- {{args}}

# Only regenerate assets/maps/overview/tiled_types.json.
gen-tiled:
    cargo run --bin gen_tiled_types

# Run the game without regenerating tiled types.
run *args:
    cargo run --bin alveus-idle-cli -- {{args}}

# Headless BRP server (realtime, no stdio) for Python drivers.
headless *args:
    cargo run --bin alveus-idle-cli --features headless -- --headless --realtime --no-stdio {{args}}

# Default-feature CI-profile tests.
test:
    cargo test --profile ci

# Headless-feature CI-profile tests (includes BRP e2e).
test-headless:
    cargo test --features headless --profile ci

# Workspace clippy with warnings denied.
clippy:
    cargo clippy --workspace -- -D warnings
