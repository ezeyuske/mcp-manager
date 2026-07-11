//! Estado del MCP built-in propio de mcp-manager.
//!
//! El built-in es el servidor `mcp-server` que se distribuye con la app.
//! Se presenta como una entrada interna: NO se puede eliminar, solo
//! activar/desactivar, y viene apagado por defecto. Activarlo registra la
//! entrada en el/los cliente(s) elegidos (Claude Code y/o Desktop) usando
//! el MISMO pipeline seguro que cualquier otra mutación
//! (`mutations::upsert`/`delete`): backup + escritura atómica + validación
//! + merge quirúrgico que preserva claves ajenas.
//!
//! `builtin.json` es estado INTERNO de la app (como `disabled.json`): se
//! escribe con `std::fs::write` plano, no con el pipeline `safe_write`
//! (que es exclusivo para configs de terceros).

use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::domain::{AppId, McpServerConfig, Scope};
use crate::error::WriteError;
use crate::mutations::{self, McpTarget};
use crate::paths;

/// Nombre reservado de la entrada del servidor propio en los configs de
/// los clientes. Un MCP de usuario con este mismo nombre en scope user
/// queda tratado como el built-in (se documenta como colisión reservada).
pub const BUILTIN_NAME: &str = "mcp-manager";

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct BuiltinState {
    /// `true` si el built-in está activo en al menos un cliente.
    pub enabled: bool,
    /// Clientes donde está registrado (siempre scope user).
    pub targets: Vec<AppId>,
    /// Path absoluto del binario `mcp-server` con el que se registró.
    /// Lo resuelve el lado Tauri (necesita ubicar el sidecar) y se guarda
    /// acá para que el inventario pueda reportarlo sin depender de Tauri.
    pub server_path: Option<String>,
}

fn read_state_at(path: &Path) -> BuiltinState {
    let Ok(raw) = std::fs::read_to_string(path) else {
        return BuiltinState::default();
    };
    if raw.trim().is_empty() {
        return BuiltinState::default();
    }
    serde_json::from_str(&raw).unwrap_or_default()
}

fn write_state_at(path: &Path, state: &BuiltinState) -> Result<(), WriteError> {
    let serialized = serde_json::to_string_pretty(state).map_err(|e| WriteError::Serialize {
        message: e.to_string(),
    })?;
    std::fs::write(path, serialized).map_err(|source| WriteError::Io {
        path: path.display().to_string(),
        source,
    })
}

/// Estado actual del built-in (default = apagado si el archivo no existe).
pub fn read_state() -> Result<BuiltinState, WriteError> {
    Ok(read_state_at(&paths::builtin_file()?))
}

/// Config del server built-in a escribir en el cliente. Sin `type` ni
/// `env`: `command`+`args` alcanzan para stdio tanto en Claude Code como
/// en Desktop, y omitir `type` evita divergencias entre clientes.
fn builtin_config(server_path: &str) -> McpServerConfig {
    McpServerConfig {
        r#type: None,
        command: Some(server_path.to_string()),
        args: Vec::new(),
        env: serde_json::Map::new(),
        url: None,
        extra: serde_json::Map::new(),
    }
}

fn target_for(app: AppId) -> McpTarget {
    McpTarget {
        app,
        scope: Scope::User,
        project_path: None,
        name: BUILTIN_NAME.to_string(),
    }
}

fn dedup(targets: &[AppId]) -> Vec<AppId> {
    let mut out: Vec<AppId> = Vec::new();
    for a in targets {
        if !out.contains(a) {
            out.push(*a);
        }
    }
    out
}

/// Diferencia entre el set de targets actual y el deseado. Devuelve
/// `(a_upsertar, a_remover)`. Los deseados siempre se upsertan (idempotente
/// y healea el path si cambió); se removen los que estaban antes y ya no.
fn reconcile(old: &[AppId], desired: &[AppId]) -> (Vec<AppId>, Vec<AppId>) {
    let to_upsert = desired.to_vec();
    let to_remove = old
        .iter()
        .filter(|a| !desired.contains(a))
        .copied()
        .collect();
    (to_upsert, to_remove)
}

/// Activa/desactiva el built-in reconciliando los configs de los clientes.
///
/// - `enabled=false` → se remueve de TODOS los targets previos; el estado
///   queda `{ enabled:false, targets:[] }`.
/// - `enabled=true` → se reconcilia a `targets`: upsert en cada uno
///   (idempotente + heal de path), delete de los que se quitaron.
///
/// El estado en `builtin.json` se persiste DESPUÉS de que las mutaciones
/// sobre los configs reales tengan éxito.
pub fn set_enabled(targets: &[AppId], enabled: bool, server_path: &str) -> Result<(), WriteError> {
    let old = read_state()?;
    let desired: Vec<AppId> = if enabled { dedup(targets) } else { Vec::new() };

    let (to_upsert, to_remove) = reconcile(&old.targets, &desired);

    // La reconciliación NO es transaccional entre clientes (cada
    // `mutations::upsert`/`delete` es atómico e independiente). Rastreamos
    // qué targets quedan efectivamente registrados y persistimos ese
    // estado incluso si una mutación intermedia falla, para que
    // `builtin.json` refleje SIEMPRE la realidad en disco (el próximo
    // toggle reconcilia lo que falte).
    let mut applied: Vec<AppId> = old.targets.clone();
    let persist = |applied: &[AppId]| -> Result<(), WriteError> {
        let state = BuiltinState {
            enabled: !applied.is_empty(),
            targets: applied.to_vec(),
            server_path: Some(server_path.to_string()),
        };
        write_state_at(&paths::builtin_file()?, &state)
    };

    for app in &to_upsert {
        if let Err(e) = mutations::upsert(&target_for(*app), builtin_config(server_path)) {
            let _ = persist(&applied);
            return Err(e);
        }
        if !applied.contains(app) {
            applied.push(*app);
        }
    }
    for app in &to_remove {
        // Si el usuario ya la borró a mano, toleramos TargetNotFound para
        // no trabar el toggle; cualquier otro error sí se propaga.
        match mutations::delete(&target_for(*app)) {
            Ok(_) | Err(WriteError::TargetNotFound { .. }) => {
                applied.retain(|a| a != app);
            }
            Err(e) => {
                let _ = persist(&applied);
                return Err(e);
            }
        }
    }

    persist(&applied)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_state_is_off() {
        let s = BuiltinState::default();
        assert!(!s.enabled);
        assert!(s.targets.is_empty());
        assert!(s.server_path.is_none());
    }

    #[test]
    fn read_state_at_missing_file_is_default_off() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("builtin.json");
        let s = read_state_at(&path);
        assert!(!s.enabled);
        assert!(s.targets.is_empty());
    }

    #[test]
    fn write_then_read_round_trips() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("builtin.json");
        let state = BuiltinState {
            enabled: true,
            targets: vec![AppId::ClaudeCode],
            server_path: Some("/abs/mcp-server".to_string()),
        };
        write_state_at(&path, &state).unwrap();
        assert_eq!(read_state_at(&path), state);
    }

    #[test]
    fn empty_file_is_default_off() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("builtin.json");
        std::fs::write(&path, "   \n").unwrap();
        assert_eq!(read_state_at(&path), BuiltinState::default());
    }

    #[test]
    fn reconcile_adds_removes_and_heals() {
        // Nada -> Code: upsert Code, nada a remover.
        let (up, rem) = reconcile(&[], &[AppId::ClaudeCode]);
        assert_eq!(up, vec![AppId::ClaudeCode]);
        assert!(rem.is_empty());

        // Code -> Desktop: upsert Desktop, remover Code.
        let (up, rem) = reconcile(&[AppId::ClaudeCode], &[AppId::ClaudeDesktop]);
        assert_eq!(up, vec![AppId::ClaudeDesktop]);
        assert_eq!(rem, vec![AppId::ClaudeCode]);

        // Code -> [] (apagar): nada a upsertar, remover Code.
        let (up, rem) = reconcile(&[AppId::ClaudeCode], &[]);
        assert!(up.is_empty());
        assert_eq!(rem, vec![AppId::ClaudeCode]);

        // Code -> Code (heal): upsert Code (idempotente), nada a remover.
        let (up, rem) = reconcile(&[AppId::ClaudeCode], &[AppId::ClaudeCode]);
        assert_eq!(up, vec![AppId::ClaudeCode]);
        assert!(rem.is_empty());
    }

    #[test]
    fn dedup_removes_duplicates_preserving_order() {
        let out = dedup(&[AppId::ClaudeCode, AppId::ClaudeCode, AppId::ClaudeDesktop]);
        assert_eq!(out, vec![AppId::ClaudeCode, AppId::ClaudeDesktop]);
    }
}

/// E2E del path completo de escritura del built-in. Redirige `HOME` a un
/// tempdir (nunca toca el `~/.claude.json` real) para verificar que
/// activar/desactivar reconcilia el config del cliente vía el pipeline
/// seguro, preservando claves ajenas y dejando backup.
#[cfg(test)]
mod e2e {
    use super::*;
    use serial_test::serial;

    /// Guarda y restaura `HOME` alrededor del test.
    struct HomeGuard {
        prev: Option<String>,
    }
    impl HomeGuard {
        fn set(dir: &std::path::Path) -> Self {
            let prev = std::env::var("HOME").ok();
            std::env::set_var("HOME", dir);
            HomeGuard { prev }
        }
    }
    impl Drop for HomeGuard {
        fn drop(&mut self) {
            match &self.prev {
                Some(v) => std::env::set_var("HOME", v),
                None => std::env::remove_var("HOME"),
            }
        }
    }

    fn read_json(path: &std::path::Path) -> serde_json::Value {
        serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
    }

    #[test]
    #[serial]
    fn enable_then_disable_claude_code_preserves_foreign_keys_and_backs_up() {
        let tmp = tempfile::tempdir().unwrap();
        let _home = HomeGuard::set(tmp.path());

        // Config previo con una clave ajena y otro MCP: deben preservarse.
        let claude_json = tmp.path().join(".claude.json");
        std::fs::write(
            &claude_json,
            r#"{"numStartups":42,"mcpServers":{"existing":{"command":"foo"}}}"#,
        )
        .unwrap();

        // ACTIVAR en Claude Code.
        set_enabled(&[AppId::ClaudeCode], true, "/abs/mcp-server").unwrap();

        let v = read_json(&claude_json);
        assert_eq!(v["mcpServers"]["mcp-manager"]["command"], "/abs/mcp-server");
        assert_eq!(v["mcpServers"]["existing"]["command"], "foo");
        assert_eq!(v["numStartups"], 42);

        let state = read_state().unwrap();
        assert!(state.enabled);
        assert_eq!(state.targets, vec![AppId::ClaudeCode]);

        // Backup del estado previo (había archivo).
        let backups = tmp.path().join(".mcp-manager/backups/claude-code");
        let has_backup = std::fs::read_dir(&backups)
            .map(|mut d| d.next().is_some())
            .unwrap_or(false);
        assert!(has_backup, "debía crearse un backup timestampeado");

        // DESACTIVAR.
        set_enabled(&[], false, "/abs/mcp-server").unwrap();
        let v2 = read_json(&claude_json);
        assert!(
            v2["mcpServers"].get("mcp-manager").is_none(),
            "la entrada built-in debe removerse al desactivar"
        );
        assert_eq!(v2["mcpServers"]["existing"]["command"], "foo");
        assert_eq!(v2["numStartups"], 42);
        assert!(!read_state().unwrap().enabled);
    }
}
