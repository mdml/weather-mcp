# AGENTS.md — weather-mcp

> Quick links: [README](README.md) · [Architecture](docs/guides/ARCHITECTURE.md) · [Development / verifier bar](docs/guides/DEVELOPMENT.md) · [Design specs](docs/design/) · [Now](docs/product/now.md) · [Roadmap](docs/product/roadmap.md) · [Decisions](docs/decisions/)

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

**Phase 1 — Design (complete) → Phase 2 next.** The Phase 0 harness **landed**
([#1](https://github.com/mdml/weather-mcp/pull/1)): a skeleton `rmcp` stdio server
(`Cargo.toml`, `src/`, `justfile`, an MCP conformance test) with green GitHub Actions CI on
`main`. The Phase 1 **design specs are now frozen** — the three tool contracts + the Phase 3
app UX are pinned in [docs/design/](docs/design/). Next is **Phase 2 — build the three tools**
against those specs (no real weather logic yet; Docker/Fly + git hooks remain deferred
follow-ups). Start at [docs/product/now.md](docs/product/now.md).

## Architecture

A single small crate (`weather-mcp`) over a transport abstraction that starts as stdio and
grows to HTTP without a rewrite, built on the official `rmcp` SDK + tokio. The Phase 0 skeleton
exists with one trivial `server_info` tool; the three real tools (`get_forecast`,
`get_historical`, `compare_period`) arrive in Phase 2. See
[ARCHITECTURE.md](docs/guides/ARCHITECTURE.md) and
[0004-minimal-tool-surface](docs/decisions/0004-minimal-tool-surface.md).

## Command surface

All dev actions go through `just` recipes (`just check`, `just test`, `just test-live`,
`just mcp-smoke`, `just run`) so agents auto-approve them (`Bash(just *)` is allowed). The
recipe set is specified in [DEVELOPMENT.md](docs/guides/DEVELOPMENT.md); `just check` is the
one-command verifier stack (fmt → clippy `-D warnings` → build → nextest).

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

Secrets are consumed **per-command**: wrap the tool that needs one in
`dotenvx run -f .env.local -- <cmd>` (e.g. `dotenvx run -f .env.local -- git push`,
`… -- gh pr create`), which decrypts via `.env.keys` and injects the secret into just that
process — they are **not** loaded into the whole session env. `gh auth setup-git` is configured
so the wrapped `git push` uses `GH_TOKEN` over HTTPS. Worktree-isolated agents don't have
`.env.local`/`.env.keys` (gitignored, absent from the worktree), so push/PR steps run from the
main checkout. **Agents must not** open the raw files (reading `.env.*` is denied) or mutate
secrets (`dotenvx set`, `just env-set` are human-only). See
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
| Open / mutate `.env.keys`, `.env.local`, `.env.*.local` | Secrets (dotenvx) | Reading is denied; consume via `dotenvx run -f .env.local -- <cmd>`. Mutation (`dotenvx set`, `just env-set`) is human-only |

## Workflow

- **Start every session at [docs/product/now.md](docs/product/now.md)** — it's the pointer to
  the current focus and the next concrete step.
- **Everything lands via a branch + PR** — `main` is protected (a GitHub ruleset requires a
  pull request + linear history), so there are no direct pushes, even for docs. CI is the real
  merge gate for code; docs PRs merge on review.
- **Substantial work runs as worktree agents that open PRs**, with a human coordinating: align
  on scope → do human-only prep (e.g. set secrets) → agents build + self-verify (`just check`) +
  open a PR → review together → merge → update [now.md](docs/product/now.md). This is the
  hands-off bet of [0005](docs/decisions/0005-hands-off-agent-development.md) in practice.
- Commit attribution: this project uses conventional-commit types incl. an `agent` type
  (e.g. `agent(harness): ...`) so the log self-documents who did what. See
  [0005-hands-off-agent-development](docs/decisions/0005-hands-off-agent-development.md).
