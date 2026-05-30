# 0007 — Secrets via dotenvx

**Date:** 2026-05-30
**Status:** active

## Context

The Open-Meteo data API is free and keyless ([0001](0001-data-source-open-meteo.md)), so the
*weather* path needs no secret. But the development and CI loop does: the first concrete need is
a **`GH_TOKEN`** so `gh` and `git push` work non-interactively (for an agent or in automation)
against the public remote `github.com/mdml/weather-mcp`.

The repo is public ([0003](0003-standalone-public-repo.md)), so the hard constraint is:
**no secret may ever land in version control, in plaintext or otherwise.** Earlier ADRs said
"no secret to store" — that holds for the *data API*; this ADR records how the *dev/CI*
credentials that do exist are handled.

## Decision

Manage secrets with **[dotenvx](https://dotenvx.com)** (encrypted-at-rest `.env` files):

| File | Contents | Committed? |
|---|---|---|
| `.env.local` | Per-user secrets (incl. `GH_TOKEN`), dotenvx-**encrypted** | No — gitignored |
| `.env.keys` | dotenvx decryption keys (plaintext) | **Never** — gitignored |
| `.env.development` | Shared config, dotenvx-**encrypted** (none yet) | Yes — encrypted, safe |

- **Mutation is human-only:** `dotenvx set GH_TOKEN <value> -f .env.local` (planned
  `just env-local-set` wrapper) encrypts the value in place.
- **Consumption:** secrets are loaded into the session environment via dotenvx so allowlisted
  tools (`gh`, `git push`) pick up `GH_TOKEN` from the env. **Agents never open the raw files.**
- **Tooling:** install the **standalone** dotenvx binary (Homebrew `dotenvx/brew/dotenvx`), not
  the npm package — keeps this repo node-free ([0002](0002-build-in-rust-with-rmcp.md)).
- **CI:** GitHub Actions uses its own encrypted **Actions secrets** (and the built-in
  `GITHUB_TOKEN`), not `.env.local`.

## Why dotenvx

- **Defense in depth.** Values are encrypted at rest, so even an accidental commit leaks only
  ciphertext, not the token. The gitignore is the primary guard; encryption is the backstop.
- **No node dependency.** The standalone binary keeps the toolchain Rust-only.
- **Simple env-injection model.** Tools consume secrets from the environment; no app code reads
  secret files directly.
- **Consistent** with the tooling this harness was adapted from.

## Guardrails

- **`.gitignore`** excludes `.env.keys` / `.env.local` / `.env.*.local`. _(in place)_
- **Agent permissions** deny *reading and writing* `.env.*`, and *mutating* secrets
  (`dotenvx set/unset/encrypt/decrypt/rotate`, `just env-set`/`env-local-set`) — in
  `.claude/settings.json`, mirrored as `forbid(...)` in `.codex/rules/default.rules`. Mutation
  is human-only. _(in place)_
- **Planned** lefthook `env-leak-guard`: block committing `.env.keys` or any unencrypted `.env*`
  (one that lacks a `DOTENV_PUBLIC_KEY` header).

## Consequences

- Contributors need `dotenvx` installed and the shared `.env.keys` delivered out-of-band (not
  via git); without the keys, encrypted values can't be decrypted.
- Reverses the literal "no secrets" phrasing in [0001](0001-data-source-open-meteo.md) and
  [0003](0003-standalone-public-repo.md) — clarified there to "no secret in *version control*."
- The dev toolchain gains a dependency (`dotenvx`); to be added to the Brewfile / `just doctor`
  in the build session.

## Alternatives considered

- **Plaintext gitignored `.env`** — rejected: one missing gitignore line or `git add -A` mistake
  leaks the token in cleartext; no encryption backstop on a public repo.
- **No file; export env vars manually each session** — rejected: not reproducible, easy to
  forget, and tempts pasting tokens into shell history.
- **`bunx dotenvx` (npm)** — rejected: pulls node/bun into an otherwise Rust-only repo; use the
  standalone binary instead.
- **SOPS / 1Password / cloud secret manager** — heavier than warranted for a single dev token;
  revisit if the secret surface grows.

## See also

- [0001 — Data source: Open-Meteo](0001-data-source-open-meteo.md)
- [0003 — Standalone public repo](0003-standalone-public-repo.md)
- [Development — Secrets](../guides/DEVELOPMENT.md#secrets-dotenvx)
