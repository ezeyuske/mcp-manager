use mcp_core::changelog::MutationLog;
use mcp_core::disabled;
use mcp_core::domain::{AppId, Inventory, McpServerConfig, Scope};
use mcp_core::mutations::{self, McpTarget};
use mcp_core::projects;
use mcp_core::safe_write;
use mcp_core::vault::{self, VaultSecretInfo};

/// Inventario unificado de MCPs. La lógica vive en `mcp_core::inventory`
/// (única fuente de verdad, compartida con la tool `list_inventory` del
/// servidor MCP). `build()` nunca falla globalmente: los errores de un
/// adapter quedan acotados a `AppInfo.error` de esa app puntual.
#[tauri::command]
pub fn get_inventory() -> Result<Inventory, String> {
    Ok(mcp_core::inventory::build())
}

#[tauri::command]
pub fn upsert_mcp(target: McpTarget, config: McpServerConfig) -> Result<(), String> {
    guard_not_builtin(&target, "editar")?;
    mutations::upsert(&target, config)
        .map(|_| ())
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_mcp(target: McpTarget) -> Result<(), String> {
    guard_not_builtin(&target, "eliminar")?;
    mutations::delete(&target)
        .map(|_| ())
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn duplicate_mcp(target: McpTarget, new_name: String) -> Result<(), String> {
    mutations::duplicate(&target, &new_name)
        .map(|_| ())
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn set_mcp_enabled(target: McpTarget, enabled: bool) -> Result<(), String> {
    let result = if enabled {
        disabled::enable(&target)
    } else {
        disabled::disable(&target)
    };
    result.map_err(|e| e.to_string())
}

#[tauri::command]
pub fn copy_mcp(
    source: McpTarget,
    dest_app: AppId,
    dest_scope: Scope,
    dest_project_path: Option<String>,
) -> Result<(), String> {
    mutations::copy(&source, dest_app, dest_scope, dest_project_path)
        .map(|_| ())
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn list_changelog() -> Result<Vec<MutationLog>, String> {
    mcp_core::changelog::list().map_err(|e| e.to_string())
}

/// Restaura un backup previamente creado sobre `target_path`: primero
/// respalda el estado ACTUAL de `target_path` (por si el usuario se
/// arrepiente de la restauración), valida que el backup sea JSON válido,
/// y luego lo copia atómicamente sobre el destino.
#[tauri::command]
pub fn restore_backup(backup_path: String, target_path: String) -> Result<(), String> {
    restore_backup_impl(&backup_path, &target_path).map_err(|e| e.to_string())
}

fn restore_backup_impl(
    backup_path: &str,
    target_path: &str,
) -> Result<(), mcp_core::error::WriteError> {
    use mcp_core::error::WriteError;

    let backup_path = std::path::Path::new(backup_path);
    let target_path = std::path::Path::new(target_path);

    let backup_raw = std::fs::read_to_string(backup_path).map_err(|source| WriteError::Io {
        path: backup_path.display().to_string(),
        source,
    })?;

    let backup_value: serde_json::Value =
        serde_json::from_str(&backup_raw).map_err(|e| WriteError::Validation {
            path: backup_path.display().to_string(),
            message: e.to_string(),
        })?;

    // `write_json` ya respalda internamente el estado ACTUAL de
    // `target_path` (si existe) antes de sobreescribirlo, así que no
    // llamamos a `safe_write::backup` explícitamente acá: hacerlo
    // generaría dos backups casi idénticos del mismo estado previo.
    safe_write::write_json(target_path, &backup_value, "restore")?;

    let log = MutationLog::new(
        // No conocemos con certeza app/scope de un restore genérico; se
        // reporta con los valores por defecto más razonables y el
        // filePath real, que es lo que el frontend necesita para
        // trazabilidad.
        AppId::ClaudeCode,
        Scope::User,
        target_path.display().to_string(),
        mcp_core::changelog::MutationAction::Restore,
        String::new(),
        Some(backup_path.display().to_string()),
    );
    let _ = mcp_core::changelog::append(log);

    Ok(())
}

#[tauri::command]
pub fn register_project_dir(path: String) -> Result<(), String> {
    projects::register(&path).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn unregister_project(path: String) -> Result<(), String> {
    projects::unregister(&path).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn list_projects() -> Result<Vec<String>, String> {
    projects::list().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_skills() -> Result<Vec<mcp_core::skills::Skill>, String> {
    mcp_core::skills::read_skills().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn set_skill_enabled(
    target: mcp_core::skills::SkillTarget,
    enabled: bool,
) -> Result<(), String> {
    mcp_core::skills::set_skill_enabled(&target, enabled).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_skill(target: mcp_core::skills::SkillTarget) -> Result<(), String> {
    mcp_core::skills::delete_skill(&target).map_err(|e| e.to_string())
}

// ---------------------------------------------------------------------
// Comandos del vault de secrets. Los VALORES nunca cruzan hacia el
// frontend salvo en `vault_reveal` (bajo demanda explícita del usuario).
// ---------------------------------------------------------------------

#[tauri::command]
pub fn vault_list() -> Result<Vec<VaultSecretInfo>, String> {
    vault::list_secrets().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn vault_set_secret(name: String, value: String) -> Result<(), String> {
    vault::set_secret(&name, &value).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn vault_delete_secret(name: String) -> Result<(), String> {
    vault::delete_secret(&name).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn vault_reveal(name: String) -> Result<String, String> {
    vault::reveal(&name).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn bind_env_secret(target: McpTarget, env_key: String, secret_name: String) -> Result<(), String> {
    vault::bind(&target, &env_key, &secret_name).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn unbind_env_secret(target: McpTarget, env_key: String) -> Result<(), String> {
    vault::unbind(&target, &env_key).map_err(|e| e.to_string())
}

/// Lee on-demand el valor INLINE de una env var (nunca precargado). Falla
/// si la clave está vault-bindeada: para esos casos el frontend usa
/// `vault_reveal`, el único canal que expone valores del keychain.
#[tauri::command]
pub fn read_env_value(target: McpTarget, env_key: String) -> Result<String, String> {
    mutations::read_env_value(&target, &env_key).map_err(|e| e.to_string())
}

/// Edición quirúrgica por-clave del `env` inline de un MCP existente:
/// aplica `upserts`/`removals` sin reemplazar el mapa completo,
/// preservando el resto del env y las claves ajenas del config.
#[tauri::command]
pub fn set_mcp_env(
    target: McpTarget,
    upserts: std::collections::HashMap<String, String>,
    removals: Vec<String>,
) -> Result<(), String> {
    guard_not_builtin(&target, "editar")?;
    let upserts_map = upserts
        .into_iter()
        .map(|(k, v)| (k, serde_json::Value::String(v)))
        .collect();
    mutations::set_mcp_env(&target, upserts_map, removals)
        .map(|_| ())
        .map_err(|e| e.to_string())
}

// ---------------------------------------------------------------------
// MCP built-in propio de la app + escritura de skills.
// ---------------------------------------------------------------------

/// Rechaza mutaciones directas sobre la entrada interna del built-in
/// (nombre reservado `mcp-manager`, scope user). Se gobierna con el toggle
/// del built-in, no con upsert/delete directos. Defensa en profundidad:
/// el frontend además oculta esas acciones.
fn guard_not_builtin(target: &McpTarget, verbo: &str) -> Result<(), String> {
    if target.scope == Scope::User && target.name == mcp_core::builtin::BUILTIN_NAME {
        return Err(format!(
            "la entrada '{}' es interna y no se puede {} directamente: usá el toggle del built-in",
            mcp_core::builtin::BUILTIN_NAME,
            verbo
        ));
    }
    Ok(())
}

/// Resuelve el path absoluto del binario sidecar `mcp-server`, ubicado
/// junto al ejecutable principal (dev: `target/<profile>/`; bundleado:
/// junto al binario de la app dentro del bundle). Ese path es lo que se
/// escribe como `command` en el config del cliente, porque es el cliente
/// (Claude) quien spawnea el proceso, no esta app.
fn resolve_sidecar_path() -> Result<String, String> {
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let dir = exe
        .parent()
        .ok_or_else(|| "no se pudo resolver el directorio del ejecutable".to_string())?;
    let name = if cfg!(windows) {
        "mcp-server.exe"
    } else {
        "mcp-server"
    };
    let candidate = dir.join(name);
    if candidate.exists() {
        Ok(candidate.to_string_lossy().to_string())
    } else {
        Err(format!(
            "no se encontró el binario mcp-server en {}",
            candidate.display()
        ))
    }
}

#[tauri::command]
pub fn get_builtin_status() -> Result<mcp_core::builtin::BuiltinState, String> {
    mcp_core::builtin::read_state().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn set_builtin_enabled(targets: Vec<AppId>, enabled: bool) -> Result<(), String> {
    let server_path = resolve_sidecar_path()?;
    mcp_core::builtin::set_enabled(&targets, enabled, &server_path).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn upsert_skill(input: mcp_core::skills::SkillInput) -> Result<(), String> {
    mcp_core::skills::upsert_skill(&input).map_err(|e| e.to_string())
}

/// Al iniciar la app: si el built-in está activo, re-resuelve el path del
/// sidecar y re-registra SOLO si cambió (p. ej. tras mover/actualizar el
/// .app). Best-effort: nunca panickea ni traba el arranque. No reescribe
/// nada si el path no cambió, para no generar backups en cada arranque.
pub fn heal_builtin_on_startup() {
    let Ok(state) = mcp_core::builtin::read_state() else {
        return;
    };
    if !state.enabled || state.targets.is_empty() {
        return;
    }
    let Ok(path) = resolve_sidecar_path() else {
        return;
    };
    if state.server_path.as_deref() == Some(path.as_str()) {
        return; // sin drift: nada que healear
    }
    if let Err(e) = mcp_core::builtin::set_enabled(&state.targets, true, &path) {
        eprintln!("mcp-manager: no se pudo re-registrar el built-in al iniciar: {e}");
    }
}
