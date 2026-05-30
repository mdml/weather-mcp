# Development — the verifier bar

This is the **spec** the build session implements ([0005](../decisions/0005-hands-off-agent-development.md)).
It defines the bar an agent grinds against; none of it is code yet.

## Toolchain

- Rust **stable** with `rustfmt` + `clippy` (pinned via `rust-toolchain.toml`).
- `rmcp` **1.7.0** + `tokio` (the MCP SDK — [0002](../decisions/0002-build-in-rust-with-rmcp.md)).
- Dev tools: `cargo-nextest`, `cargo-deny`, `cargo-audit`, `insta`, `just`.
- `dotenvx` for secrets (install the standalone binary — Homebrew `dotenvx/brew/dotenvx`, not
  the npm package — to keep the repo node-free). See [Secrets](#secrets-dotenvx).

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

## Secrets (dotenvx)

The Open-Meteo data API is keyless ([0001](../decisions/0001-data-source-open-meteo.md)), but
the dev/CI loop still needs credentials — currently a **`GH_TOKEN`** so `gh` / `git push` work
non-interactively. Secrets are managed with [dotenvx](https://dotenvx.com) and
[ADR 0007](../decisions/0007-secrets-via-dotenvx.md):

| File | Contents | Committed? |
|---|---|---|
| `.env.local` | Per-user secrets (incl. `GH_TOKEN`), dotenvx-**encrypted** | No — gitignored |
| `.env.keys` | dotenvx decryption keys (plaintext) | **Never** — gitignored |
| `.env.development` | Shared config, dotenvx-**encrypted** (none yet) | Yes — encrypted, safe |

- **Set a secret (human-only):** `dotenvx set GH_TOKEN <value> -f .env.local` (planned
  `just env-local-set <key> <value>` wrapper). This encrypts the value in `.env.local`.
- **Consume:** secrets are loaded into the session environment via dotenvx, so allowlisted
  tools (`gh`, `git push`) read `GH_TOKEN` from the env. Agents never open the raw files.
- **Guardrails:**
  - `.gitignore` excludes `.env.keys` / `.env.local` / `.env.*.local`.
  - Agent permissions deny **reading and writing** `.env.*` and **mutating** secrets
    (`dotenvx set/unset/encrypt/decrypt/rotate`, `just env-set`/`env-local-set`) — in
    `.claude/settings.json`, mirrored as `forbid(...)` in `.codex/rules/default.rules`. Mutation
    is human-only.
  - **Planned** lefthook `env-leak-guard`: block committing `.env.keys` or any unencrypted
    `.env*` (no `DOTENV_PUBLIC_KEY` header).
- **CI:** GitHub Actions uses its own encrypted **Actions secrets**, not `.env.local`. The
  built-in `GITHUB_TOKEN` covers most needs; a PAT goes in an Actions secret if required.

## Conventions

- Commit messages: conventional commits incl. an `agent` type (e.g. `agent(harness): wire just check`).
- Agents work in worktrees; `.scratch/` for ephemeral artifacts.
- New repeatable command? Add a `just` recipe rather than running it ad hoc.
