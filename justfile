# weather-mcp — the command surface (DEVELOPMENT.md).
# Every repeatable dev action is a recipe so agents auto-approve `just <recipe>`.

# Show the recipe list by default.
default:
    @just --list

# The full local verifier stack, in order (ADR 0005). One command = "is this right?".
# fmt -> clippy(-D warnings) -> build -> nextest (incl. insta snapshots + conformance).
check:
    cargo fmt --check
    cargo clippy --all-targets -- -D warnings
    cargo build
    cargo nextest run

# Offline + deterministic test run (insta snapshots + the MCP conformance test).
test:
    cargo nextest run

# Live-API smoke test against Open-Meteo. Placeholder in Phase 0 — there is no upstream
# client yet (src/openmeteo/ is empty). Kept present so the recipe surface is stable.
test-live:
    @echo "[test-live] no upstream Open-Meteo client yet (Phase 1) — nothing to hit."

# Spawn the built server over stdio and run a real MCP session:
# initialize -> tools/list -> tools/call server_info. The verifier agents can't mock this.
# It's the same end-to-end path as the conformance test, runnable on demand.
mcp-smoke:
    cargo nextest run --test conformance

# Run the server over stdio (for manual / Claude Desktop use).
run:
    cargo run
