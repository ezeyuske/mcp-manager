# mcp-manager

A desktop app to manage **MCP servers** (and Claude **skills**) across **Claude Desktop**
and **Claude Code** — safely. Built with **Tauri v2** (Rust) + **React 19 / TypeScript**.

MCP servers live in a handful of JSON config files scattered across apps, scopes, and
projects. Editing them by hand is error-prone, and a corrupted config is silent until
the host app fails to start. mcp-manager gives you one place to see and edit all of
them, with a safety model built so it never leaves a foreign config broken.

## What it does

- **Unified inventory** — every MCP installation across all supported apps and scopes
  in one list, with transport (stdio / HTTP / SSE) and a health status per entry.
- **MCP CRUD** — add, edit, duplicate, enable/disable, and copy entries across apps
  and scopes.
- **Env vars + secrets vault** — secret values are stored in the **OS keychain**, never
  on disk or in the frontend. Secrets are bound to an MCP's env var **per-MCP, opt-in**
  (least privilege — a secret is only injected into the MCPs you explicitly bind it to),
  and injected at write time.
- **Per-project scopes** — register your repos and manage user-scope vs. project-scope
  (`.mcp.json`) MCPs side by side.
- **Skills management** — enable, disable, and delete Claude skills **non-destructively**:
  disable/delete are reversible moves to sidecar directories, not real deletions.
- **Activity log + restore** — an audit trail of every mutation, and one-click restore
  from timestamped backups.

## Safety model

This app writes to config files owned by *other* apps, so corrupting one is treated as
the worst possible bug. Every write to a foreign config goes through the same pipeline
(`src-tauri/src/safe_write.rs`, `src-tauri/src/mutations.rs`):

1. **Timestamped backup** of the current file → `~/.mcp-manager/backups/`
2. **Atomic write** (write to a temp file in the same dir, then `rename`)
3. **JSON re-validation** — the serialized result is reparsed *before* it touches disk;
   invalid JSON aborts the write
4. **Surgical merge** — only known fields are touched; unknown/unmodeled keys are
   preserved verbatim (never rewrite a file from a partial model)

App state lives under `~/.mcp-manager/`: `backups/`, `vault.json` (secret *names* and
bindings only — never values), `changelog.json`, `projects.json`, and the
disabled/deleted-skills sidecars.

## Supported targets

| App | Config | OS |
|-----|--------|----|
| **Claude Desktop** | `~/Library/Application Support/Claude/claude_desktop_config.json` (macOS), `%APPDATA%/Claude/claude_desktop_config.json` (Windows) | macOS, Windows — **not Linux** |
| **Claude Code** | `~/.claude.json` (user scope) + `<project>/.mcp.json` (project scope) | All |

Each target is an implementation of the `AppAdapter` trait; OS paths are resolved with
the `dirs` crate (never hardcoded). Where an app doesn't exist on a platform, the adapter
reports "not installed" rather than inventing a path.

## Tech stack

- **Tauri 2** (capabilities-based permissions, no allowlist)
- **React 19** + **TypeScript** (strict)
- **Tailwind v4** — CSS-first via `@theme` (no `tailwind.config.js`); design tokens in
  `src/styles/theme.css`, runtime-configurable accent
- **Zustand** for frontend state
- Rust crates: `keyring` (OS keychain), `dirs` (paths), `which` (binary detection),
  `serde`/`serde_json`, `chrono`, `tempfile`
- **pnpm** as the package manager (respect `pnpm-lock.yaml` — never npm/yarn)

## Getting started

Prerequisites:

- A Rust toolchain (`rustup`)
- `pnpm`
- [Tauri v2 system dependencies](https://v2.tauri.app/start/prerequisites/) for your OS

```bash
pnpm install
pnpm tauri dev
```

## Commands

| Command | What it does |
|---------|--------------|
| `pnpm tauri dev` | Run the full desktop app (Vite on port 1420, then the Tauri window). Use this to exercise the Rust side. |
| `pnpm dev` | Vite frontend only, in a browser. `invoke` calls fail (no Rust backend); stores fall back to mock data. |
| `pnpm build` | Type-check (`tsc`) then build the frontend bundle to `dist/`. |
| `pnpm tauri build` | Produce distributable desktop binaries. |

Rust checks run inside `src-tauri/`:

```bash
cd src-tauri
cargo check
cargo clippy -- -D warnings
```

## Architecture

The frontend↔backend boundary is the thing to understand: **the frontend never touches
config files directly.** All config access goes through typed Tauri commands (~18 of
them, registered in `src-tauri/src/lib.rs`); `plugin-fs`/`plugin-dialog` are only used
for native file pickers. Each command routes through an `AppAdapter` per target.

```
src/
├── App.tsx              # Shell layout + store-based screen routing (no URL router)
├── screens/             # mcps, skills, projects, env+secrets, activity, themes, settings
├── components/          # glassmorphic design-system components (Button, Toggle, Modal, …)
├── store/               # Zustand stores (inventory, vault, skills, projects, mutations, …)
├── types/               # domain types mirroring the Rust DTOs
└── styles/theme.css     # Tailwind v4 @theme tokens + runtime accent CSS vars

src-tauri/src/
├── lib.rs               # entry point, plugin init, command registration
├── commands.rs          # the Tauri commands
├── adapters/            # AppAdapter trait + claude_desktop.rs, claude_code.rs
├── safe_write.rs        # atomic write + backup + JSON validation
├── mutations.rs         # surgical merge + vault-aware env injection
├── vault.rs             # OS keychain integration + per-MCP bindings
├── skills.rs            # non-destructive skill enable/disable/delete
└── projects.rs          # project registry (projects.json)
```

## Project status

Five phases are shipped: (1) design system + shell, (2) read-only adapters + unified
inventory, (3) MCP CRUD + safe writes + log/restore, (4) env vars + keychain vault,
(5) per-project scopes + skills management.

There is no JS-side test runner yet; the Rust side has `#[cfg(test)]` tests with fixtures,
including round-trip checks that verify foreign keys are preserved.

Contributors: read `CLAUDE.md` for the project's non-negotiable rules (config safety,
architecture, stack/versions, design system). The design system's single source of truth
is `.claude/skills/mcp-manager-design/`.
