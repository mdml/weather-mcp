# Development — the verifier bar

This is the **spec** the build session implements ([0005](../decisions/0005-hands-off-agent-development.md)).
It defines the bar an agent grinds against; none of it is code yet.

## Toolchain

- Rust **stable** with `rustfmt` + `clippy` (pinned via `rust-toolchain.toml`).
- `rmcp` **1.7.0** + `tokio` (the MCP SDK — [0002](../decisions/0002-build-in-rust-with-rmcp.md)).
- Dev tools: `cargo-nextest`, `cargo-deny`, `cargo-audit`, `insta`, `just`.

## `just` recipes (the command surface)

Wrap every repeatable dev action in a recipe so agents auto-approve it (`Bash(just *)` is
allowed). Planned set:

| Recipe | Does | Notes |
|---|---|---|
| `just check` | The full local verifier stack (below), in order | One command = "is this right?" |
| `just test` | `cargo nextest run` | Offline + deterministic (fixtures) |
| `just test-live` | Live-API smoke test against Open-Meteo | Network; catches upstream drift |
| `just mcp-smoke` | Spawn the server over stdio, run the conformance script | The verifier agents can't mock |
| `just run` | Run the server (stdio) | For manual / Claude Desktop use |

## `just check` — the verifier stack (in order)

1. `cargo fmt --check`
2. `cargo clippy -- -D warnings`
3. `cargo build`
4. `cargo nextest run`
5. `insta` snapshot tests — pin tool-output shapes (`tools/list` schema, tool result JSON)
6. **MCP conformance test** — spawn the server over stdio and script a real client session:
   `initialize` → `tools/list` → `tools/call`. **This is the verifier that can't be gamed with
   mocks** — build it (and `test-live`) *first* so the rest of the suite can't be satisfied by
   mocking away the real work.

## Testing strategy — mock the outermost boundary, not collaborators

- **Fixtures:** record real Open-Meteo responses (forecast + archive) and commit them. Run all
  parsing/aggregation against the fixtures — offline and deterministic. This is the bulk of
  `cargo nextest run`.
- **Thin live layer:** `just test-live` hits the real API to catch upstream drift. Keep it thin
  and separate from the deterministic suite.
- **Conformance over stdio:** the end-to-end check that the server actually speaks MCP.
- **The trap:** tests that pass but don't exercise real behavior. Mitigation is structural —
  the conformance + live tests exist first, so mock-only tests can't masquerade as coverage.

## CI — the real merge gate

Public repo → free GitHub Actions ([0003](../decisions/0003-standalone-public-repo.md)). On
every push:

- `cargo fmt --check`
- `cargo clippy -- -D warnings`
- `cargo nextest run`
- `cargo build`
- `cargo-deny` + `cargo-audit` (supply-chain story)
- Docker image build; Fly.io preview per branch (deploy is human-in-loop)

The human reviews a green checkmark, not every line.

## Conventions

- Commit messages: conventional commits incl. an `agent` type (e.g. `agent(harness): wire just check`).
- Agents work in worktrees; `.scratch/` for ephemeral artifacts.
- New repeatable command? Add a `just` recipe rather than running it ad hoc.
