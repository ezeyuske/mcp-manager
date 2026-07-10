use crate::adapters::all_adapters;
use crate::changelog::MutationLog;
use crate::disabled;
use crate::domain::{AppId, Inventory, McpServerConfig, McpStatus, Scope};
use crate::mutations::{self, McpTarget};
use crate::projects;
use crate::safe_write;

/// Recorre todos los adapters de apps soportadas y arma un inventario
/// unificado. Si un adapter falla al leer su config (JSON corrupto,
/// error de I/O), el error queda acotado a `AppInfo.error` de esa app
/// puntual y el resto del inventario se arma igual: nunca devolvemos
/// `Err` global por un problema de una sola app.
///
/// Además de lo que reportan los adapters, se suman:
/// - Las entradas actualmente deshabilitadas (sidecar `disabled.json`),
///   con `status: disabled` y `enabled: false`.
/// - Las entradas de los `.mcp.json` de proyectos registrados
///   (`projects.json`), como scope Project de Claude Code.
#[tauri::command]
pub fn get_inventory() -> Result<Inventory, String> {
    let mut apps = Vec::new();
    let mut installations = Vec::new();

    for adapter in all_adapters() {
        let mut info = adapter.detect();

        if info.installed {
            match adapter.read() {
                Ok(mut found) => installations.append(&mut found),
                Err(err) => info.error = Some(err.to_string()),
            }
        }

        apps.push(info);
    }

    // Proyectos registrados: leemos su .mcp.json standalone (si existe).
    if let Ok(project_paths) = projects::list() {
        for project_path in project_paths {
            let mcp_json_path = std::path::Path::new(&project_path).join(".mcp.json");
            if !mcp_json_path.exists() {
                continue;
            }

            let Ok(raw) = std::fs::read_to_string(&mcp_json_path) else {
                continue;
            };
            let Ok(file) = serde_json::from_str::<crate::domain::McpJsonFile>(&raw) else {
                continue;
            };

            for (name, value) in file.mcp_servers.iter() {
                let Ok(cfg) = serde_json::from_value::<McpServerConfig>(value.clone()) else {
                    continue;
                };
                installations.push(crate::adapters::build_installation(
                    name,
                    AppId::ClaudeCode,
                    Scope::Project,
                    Some(project_path.clone()),
                    cfg,
                    mcp_json_path.display().to_string(),
                ));
            }
        }
    }

    // Entradas deshabilitadas: se muestran con status Disabled y
    // enabled=false, para que el frontend pueda listarlas/reactivarlas.
    if let Ok(disabled_entries) = disabled::list_disabled() {
        for (_key, entry) in disabled_entries {
            let cfg: McpServerConfig =
                serde_json::from_value(entry.config.clone()).unwrap_or(McpServerConfig {
                    r#type: None,
                    command: None,
                    args: Vec::new(),
                    env: serde_json::Map::new(),
                    url: None,
                    extra: serde_json::Map::new(),
                });

            let target = McpTarget {
                app: entry.app,
                scope: entry.scope,
                project_path: entry.project_path.clone(),
                name: entry.name.clone(),
            };
            let config_path = mutations::resolve_target_path(&target)
                .map(|p| p.display().to_string())
                .unwrap_or_default();

            let mut installation = crate::adapters::build_installation(
                &entry.name,
                entry.app,
                entry.scope,
                entry.project_path,
                cfg,
                config_path,
            );
            installation.status = McpStatus::Disabled;
            installation.enabled = false;
            installations.push(installation);
        }
    }

    Ok(Inventory {
        apps,
        installations,
    })
}

#[tauri::command]
pub fn upsert_mcp(target: McpTarget, config: McpServerConfig) -> Result<(), String> {
    mutations::upsert(&target, config)
        .map(|_| ())
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_mcp(target: McpTarget) -> Result<(), String> {
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
    crate::changelog::list().map_err(|e| e.to_string())
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
) -> Result<(), crate::error::WriteError> {
    use crate::error::WriteError;

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
        crate::changelog::MutationAction::Restore,
        String::new(),
        Some(backup_path.display().to_string()),
    );
    let _ = crate::changelog::append(log);

    Ok(())
}

#[tauri::command]
pub fn register_project_dir(path: String) -> Result<(), String> {
    projects::register(&path).map_err(|e| e.to_string())
}
