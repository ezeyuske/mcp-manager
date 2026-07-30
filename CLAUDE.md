# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What this is

Desktop app ("mcp-manager") built with **Tauri v2** (Rust backend) + **React 19 / TypeScript** (Vite frontend). It manages MCP servers (and Claude skills) across Claude Desktop and Claude Code. Five phases are shipped: design system + shell, read-only adapters + unified inventory, MCP CRUD with safe writes + log/restore, env vars + keychain vault, and per-project scopes + skills management. See `README.md` for the feature and architecture overview. Package manager is **pnpm** (see `packageManager` in `package.json`).

## Commands

- `pnpm tauri dev` — run the full desktop app (starts Vite on port 1420, then the Tauri window). Use this, not `pnpm dev`, to exercise the Rust side.
- `pnpm dev` — Vite frontend only, in a browser (Tauri `invoke` calls will fail — no Rust backend).
- `pnpm build` — type-check (`tsc`) then build the frontend bundle to `dist/`.
- `pnpm tauri build` — produce distributable desktop binaries.
- Rust checks live under `src-tauri/`: run `cargo check` / `cargo clippy` there (no npm script wraps them).

There is no test runner or linter configured yet.

### Sandbox de desarrollo (aislamiento de configs reales)

Setear `MCP_MANAGER_CONFIG_ROOT=<dir>` redirige TODA la resolución de paths
de la app a ese directorio, para que `pnpm tauri dev` no toque los configs
reales del usuario. Layout: `<dir>/home` reemplaza a `dirs::home_dir()`
(`~/.claude.json`, `~/.claude/skills`, `~/.mcp-manager`) y `<dir>/config`
reemplaza a `dirs::config_dir()` (config de Claude Desktop). El keychain,
que no es un archivo, se aísla usando un service separado
(`mcp-manager-sandbox`). Al arrancar con el sandbox activo, el proceso
imprime un banner por stderr; si NO aparece, no estás aislado (revisá el
nombre de la env var). Toda la lógica vive en `crates/mcp-core/src/paths.rs`
(`home_dir` / `config_dir` / `sandbox_active`); no llamar `dirs::` directo
desde adapters ni mutations — siempre vía `paths::`.

## Architecture

The frontend↔backend boundary is the key thing to understand:

- **Frontend** (`src/`) calls Rust via `invoke("command_name", args)` from `@tauri-apps/api/core`. See the Zustand stores in `src/store/` (e.g. `inventory.ts`, `mutations.ts`) for how commands are called.
- **Backend** (`src-tauri/src/lib.rs`) exposes functions annotated with `#[tauri::command]`. Every command **must** be registered in the `tauri::generate_handler![...]` macro inside `run()`, or `invoke` will fail at runtime. `main.rs` is a thin entry point that just calls `lib.rs::run()`.
- **Permissions**: Tauri v2 gates all backend/plugin capability behind `src-tauri/capabilities/default.json`. When adding a plugin (fs, dialog, shell) or a feature that needs OS access, add the corresponding permission there — otherwise calls are silently blocked. Plugins are also declared in `src-tauri/Cargo.toml` (Rust) and initialized with `.plugin(...)` in `lib.rs`.

Wired plugins: `tauri-plugin-opener`, `tauri-plugin-fs`, `tauri-plugin-dialog`, and `tauri-plugin-shell` are declared in `Cargo.toml`, initialized with `.plugin(...)` in `lib.rs`, and granted permissions in `capabilities/default.json` (`opener:default`, `fs:default`, `dialog:default`, `shell:allow-open`). When adding another plugin, wire all three layers (Cargo init + capability + JS import).

## Gotchas

- **Tailwind v4 is active, CSS-first.** `@tailwindcss/vite` is wired in `vite.config.ts` and `src/styles/theme.css` holds `@import "tailwindcss"` plus the `@theme` token block and runtime accent CSS vars. There is no `tailwind.config.js` — configure via `@theme`, not a JS config.
- `vite.config.ts` pins port 1420 with `strictPort: true` — the dev server fails rather than falling back to another port. Tauri depends on this fixed port.

## Reglas del proyecto (no negociables)

### Seguridad de configs ajenos
- Esta app escribe en archivos de configuración de OTRAS apps. Corromper
  un config es el peor bug posible del proyecto.
- Toda escritura a configs externos: backup timestampeado previo en
  ~/.mcp-manager/backups/ + escritura atómica (tmp + rename) + validación
  del JSON resultante antes del rename.
- Merge quirúrgico SIEMPRE: preservar claves desconocidas. Los structs de
  config llevan captura de claves no modeladas (`#[serde(flatten)]` sobre
  un mapa). Prohibido reescribir un archivo desde un modelo parcial.
- `~/.claude.json` contiene mucho más que MCPs (historial, settings):
  tratarlo con máximo cuidado.
- Después de tocar cualquier adapter o código de escritura de configs,
  invocar al subagente config-safety-reviewer.

### Arquitectura
- Frontend NUNCA accede a los configs directamente: todo pasa por
  commands de Tauri tipados. plugin-fs solo para dialogs de archivos.
- Un adapter por app-target implementando el trait `AppAdapter`
  (paths por OS, read_config, write_config, formato de MCP propio).
- Paths por OS con el crate `dirs`. Claude Desktop no existe en Linux:
  el adapter devuelve NotInstalled, no un path inventado.
- Secrets solo en keychain del OS (crate `keyring`), inyectados al
  escribir. Nunca en texto plano en el estado del frontend ni en logs.
  En Linux sin Secret Service: degradar con error claro, no panic.

### Stack y versiones
- Tauri 2 (NO Tauri 1: capabilities, no allowlist) + React 19 + TS
  estricto + Tailwind v4 (NO tailwind.config.js: configuración CSS-first
  con @theme). Ante duda sobre APIs de Tauri 2 o Tailwind v4, consultar
  context7 antes de escribir código.
- Package manager: pnpm SIEMPRE (nunca npm/yarn — respetar pnpm-lock.yaml).
- Estado del frontend con Zustand. Tipos compartidos de dominio en
  src/types/.

### Design system
- Tokens en CSS variables; el acento es configurable en runtime, nunca
  hardcodear coral en componentes.
- Al terminar tareas de UI, invocar al subagente ui-reviewer.
- Referencia completa del estilo: .claude/skills/mcp-manager-design/
  (cuando exista; hasta entonces, la sección de estilo del prompt inicial).

### Calidad
- `cargo clippy -- -D warnings` y `tsc --noEmit` limpios antes de dar
  por cerrada cualquier tarea.
- Cambios de parsing/merge de configs requieren tests con fixtures,
  incluyendo round-trip que verifique preservación de claves ajenas.