//! Ensamblado del inventario unificado de MCPs.
//!
//! Única fuente de verdad, reutilizada por el command Tauri `get_inventory`
//! y por la tool `list_inventory` del servidor MCP standalone, para que la
//! GUI y el LLM vean exactamente lo mismo.

use crate::adapters::{all_adapters, build_installation};
use crate::builtin::BUILTIN_NAME;
use crate::domain::{AppId, AppInfo, Inventory, McpInstallation, McpServerConfig, McpStatus, Scope};
use crate::mutations::{self, McpTarget};
use crate::{disabled, projects, vault};

/// Recorre todos los adapters de apps soportadas y arma un inventario
/// unificado. Si un adapter falla al leer su config (JSON corrupto, error
/// de I/O), el error queda acotado a `AppInfo.error` de esa app puntual y
/// el resto del inventario se arma igual: nunca devolvemos `Err` global.
///
/// Además de lo que reportan los adapters, se suman:
/// - Las entradas actualmente deshabilitadas (sidecar `disabled.json`),
///   con `status: disabled` y `enabled: false`.
/// - Las entradas de los `.mcp.json` de proyectos registrados
///   (`projects.json`), como scope Project de Claude Code.
///
/// Por último, se marca `builtin: true` en la entrada propia de la app
/// (nombre reservado `mcp-manager`, scope user) si el usuario la activó: el
/// frontend la excluye de la lista normal y la gobierna con su card.
pub fn build() -> Inventory {
    let mut apps: Vec<AppInfo> = Vec::new();
    let mut installations: Vec<McpInstallation> = Vec::new();

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
                installations.push(build_installation(
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

            let mut installation = build_installation(
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

    // Cruzamos con el vault para poblar `vault_keys`: subconjunto de
    // `env_keys` que tiene un binding activo. Se hace acá (y no dentro de
    // `build_installation`) para que `adapters` no necesite conocer el
    // vault; si el vault no está disponible, la instalación queda con
    // `vault_keys` vacío en vez de tumbar el inventario entero.
    for installation in installations.iter_mut() {
        let target = McpTarget {
            app: installation.app,
            scope: installation.scope,
            project_path: installation.project_path.clone(),
            name: installation.name.clone(),
        };
        if let Ok(bindings) = vault::bindings_for_target(&target) {
            installation.vault_keys = bindings.into_iter().map(|(env_key, _)| env_key).collect();
        }
    }

    // Marcar la entrada propia del built-in: si está activa, los adapters
    // la leyeron como un MCP normal llamado `mcp-manager` en scope user.
    for installation in installations.iter_mut() {
        if installation.scope == Scope::User && installation.name == BUILTIN_NAME {
            installation.builtin = true;
        }
    }

    Inventory { apps, installations }
}
