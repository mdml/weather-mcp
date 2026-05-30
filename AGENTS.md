# AGENTS.md — weather-mcp

> Quick links: [README](README.md) · [Architecture](docs/guides/ARCHITECTURE.md) · [Development / verifier bar](docs/guides/DEVELOPMENT.md) · [Now](docs/product/now.md) · [Roadmap](docs/product/roadmap.md) · [Decisions](docs/decisions/)

## What this is

A standalone, open-source **Rust MCP server** wrapping the [Open-Meteo](https://open-meteo.com)
API. It answers both *"what's the forecast"* and *"how does this year's rainfall compare to
the past decade"* — the historical-trend question no consumer weather app exposes.

The guiding bet: **hands-off development with agents, favoring verifiers over approval
gates.** Every "is this right?" should be answerable by a command, not by a human reading a
diff. Rust is chosen partly because the compiler + clippy + a borrow checker are a brutal,
free verifier loop an agent can self-correct against. See
[0005-hands-off-agent-development](docs/decisions/0005-hands-off-agent-development.md).

## Status

**Early — harness/docs only.** This repo currently contains decision records, a roadmap,
guides, and the agent config. There is **no Rust code yet** (`Cargo.toml`, `src/`, `justfile`,
CI, Docker are intentionally not present). The next session scaffolds the skeleton server +
verifier stack + CI — start at [docs/product/now.md](docs/product/now.md).

## Architecture (planned)

A single small crate exposing three tools (`get_forecast`, `get_historical`,
`compare_period`) over a transport abstraction that starts as stdio and grows to HTTP without
a rewrite. Built on the official `rmcp` SDK + tokio. See
[ARCHITECTURE.md](docs/guides/ARCHITECTURE.md) and
[0004-minimal-tool-surface](docs/decisions/0004-minimal-tool-surface.md).

## Command surface (planned)

Not built yet. Once the harness lands, all dev actions go through `just` recipes
(`just check`, `just test`, `just test-live`, `just mcp-smoke`, `just run`) so agents
auto-approve them. The exact recipe set is specified in
[DEVELOPMENT.md](docs/guides/DEVELOPMENT.md). Until then, use `cargo`/`git` directly.

## Supported agents

Both **Claude Code** and **Codex**. Config is committed and hand-maintained (no generator):

- Claude: [`.claude/settings.json`](.claude/settings.json) (permission allow/ask/deny matrix)
- Codex: [`.codex/config.toml`](.codex/config.toml) + [`.codex/rules/default.rules`](.codex/rules/default.rules)
- Repo MCP servers: [`.mcp.json`](.mcp.json) (empty for now)

**Keep the two matrices in sync by hand.** If you add a permission to `.claude/settings.json`,
add the mirrored `allow(...)`/`forbid(...)` line to `.codex/rules/default.rules`.

## Secrets (dotenvx)

Secrets are managed with [dotenvx](https://dotenvx.com) and **never committed in plaintext**:

- **`.env.local`** — per-user local secrets, dotenvx-**encrypted**, **gitignored**. Holds e.g.
  `GH_TOKEN` (a GitHub token so `gh` / `git push` work non-interactively).
- **`.env.keys`** — the dotenvx decryption keys, plaintext, **gitignored**, **never commit**.
- A committed, dotenvx-**encrypted** `.env.development` may hold shared config (none yet).

Secrets are loaded into the session environment (via dotenvx) so allowlisted tools (`gh`,
`git push`) pick up `GH_TOKEN` automatically. **Agents must not** open the raw files (reading
`.env.*` is denied) or mutate secrets (`dotenvx set`, `just env-set` are human-only). See
[0007](docs/decisions/0007-secrets-via-dotenvx.md) and
[DEVELOPMENT.md](docs/guides/DEVELOPMENT.md#secrets-dotenvx).

## Agent conventions

- **Don't chain git commands with `&&`** — run them as separate calls.
- **`cd` doesn't persist** between tool calls and is denied; use absolute paths instead.
- **Use `.scratch/`** for ephemeral artifacts (scratch files, throwaway scripts).
- **Prefer `just <recipe>`** over direct tool invocation once the justfile exists; when you
  need a new repeatable command, add a recipe rather than running it ad hoc.
- **Network fetches** go through the `WebFetch` tool, not `curl`/`wget` (both denied). The
  allowed doc domains are listed in `.claude/settings.json`.

### Permission guards

| Blocked | Why | Do this instead |
|---|---|---|
| `curl`, `wget` | Unbounded network egress | Use the `WebFetch` tool |
| `cd`, `pushd`, `popd`, `git -C` | cwd doesn't persist across calls | Use absolute paths |
| `rm -rf`, `rm -r` | Irreversible recursive delete | Remove files individually, or ask |
| `git push --force` / `-f`, `git reset --hard`, `git clean -f` | Destructive history/working-tree loss | `--force-with-lease` (prompts), or ask |
| `git commit --no-verify`, `git push --no-verify` | Skips the hooks that exist for a reason | Fix the failing check |
| Open / mutate `.env.keys`, `.env.local`, `.env.*.local` | Secrets (dotenvx) | Reading is denied; consume via the loaded env (`GH_TOKEN`). Mutation (`dotenvx set`, `just env-set`) is human-only |

## Workflow

- **Start every session at [docs/product/now.md](docs/product/now.md)** — it's the pointer to
  the current focus and the next concrete step.
- Code lands on feature branches with green CI (the real gate); docs can go direct to `main`.
- Commit attribution: this project uses conventional-commit types incl. an `agent` type
  (e.g. `agent(harness): ...`) so the log self-documents who did what. See
  [0005-hands-off-agent-development](docs/decisions/0005-hands-off-agent-development.md).
