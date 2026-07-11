use std::path::Path;

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::changelog::{self, MutationAction};
use crate::domain::AppId;
use crate::domain::Scope;
use crate::error::WriteError;
use crate::mutations::{self, McpTarget};
use crate::paths::disabled_file;

/// Entrada guardada en el sidecar de deshabilitados: conserva el `Value`
/// crudo tal cual vivía en el config de origen, para poder restaurarlo
/// exactamente igual al habilitar.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DisabledEntry {
    pub app: AppId,
    pub scope: Scope,
    pub project_path: Option<String>,
    pub name: String,
    pub config: Value,
}

fn sidecar_key(target: &McpTarget) -> String {
    format!(
        "{}|{}|{}|{}",
        serde_json::to_value(target.app).unwrap_or(Value::Null),
        serde_json::to_value(target.scope).unwrap_or(Value::Null),
        target.project_path.as_deref().unwrap_or(""),
        target.name
    )
}

fn read_all_at(path: &Path) -> Result<Map<String, Value>, WriteError> {
    if !path.exists() {
        return Ok(Map::new());
    }

    let raw = std::fs::read_to_string(path).map_err(|source| WriteError::Io {
        path: path.display().to_string(),
        source,
    })?;

    if raw.trim().is_empty() {
        return Ok(Map::new());
    }

    let value: Value = serde_json::from_str(&raw).map_err(|e| WriteError::Validation {
        path: path.display().to_string(),
        message: e.to_string(),
    })?;

    Ok(value.as_object().cloned().unwrap_or_default())
}

/// Escribe el sidecar completo con `std::fs::write` directo (sin pasar
/// por `safe_write`). Esto es intencional: `disabled.json` es estado
/// INTERNO de mcp-manager (igual que `changelog.json`, ver comentario en
/// `changelog.rs`), no un config de una app de terceros. No necesita el
/// backup timestampeado ni la escritura atómica que reservamos para los
/// configs ajenos que este proyecto no controla — si se corrompiera, el
/// peor caso es perder el registro de qué estaba deshabilitado, nunca
/// dañar un config real de otra app.
fn write_all_at(path: &Path, entries: &Map<String, Value>) -> Result<(), WriteError> {
    let serialized =
        serde_json::to_string_pretty(entries).map_err(|e| WriteError::Serialize {
            message: e.to_string(),
        })?;

    std::fs::write(path, serialized).map_err(|source| WriteError::Io {
        path: path.display().to_string(),
        source,
    })
}

/// Mueve la entrada `target` del config real al sidecar de deshabilitados:
/// lee su `Value` actual, borra la entrada del config de origen (con
/// backup, vía `mutations::delete`), y SOLO si eso tuvo éxito la guarda
/// en `disabled.json`.
///
/// ORDEN DELIBERADO (config real primero, sidecar después): si
/// escribiéramos el sidecar antes de borrar del config real y el
/// proceso muriera en el medio, la entrada quedaría DUPLICADA (viva en
/// el config real y también en el sidecar). Con este orden, si el
/// proceso muere entre el `delete` y el `write_all_at` del sidecar, el
/// peor caso es que la entrada quede "perdida" (ni en el config ni en
/// el sidecar) — pero sigue siendo recuperable desde el backup
/// timestampeado que `mutations::delete` ya generó. Es preferible una
/// entrada temporalmente perdida-pero-recuperable a una duplicada.
pub fn disable(target: &McpTarget) -> Result<(), WriteError> {
    disable_with_sidecar(&disabled_file()?, target)
}

fn disable_with_sidecar(sidecar_path: &Path, target: &McpTarget) -> Result<(), WriteError> {
    let raw_config = mutations::read_entry_value(target)?;

    let backup_path = mutations::delete(target)?;

    let entry = DisabledEntry {
        app: target.app,
        scope: target.scope,
        project_path: target.project_path.clone(),
        name: target.name.clone(),
        config: raw_config,
    };

    let mut entries = read_all_at(sidecar_path)?;
    entries.insert(
        sidecar_key(target),
        serde_json::to_value(&entry).map_err(|e| WriteError::Serialize {
            message: e.to_string(),
        })?,
    );
    // Si esto falla, la entrada ya salió del config real (con su backup
    // a salvo) pero no llegó a guardarse en el sidecar: se pierde el
    // estado "disabled", no el contenido de la entrada. Propagamos el
    // error para que el frontend lo sepa en vez de reportar éxito falso.
    write_all_at(sidecar_path, &entries)?;

    let file_path = mutations::resolve_target_path(target)?;
    let log = crate::changelog::MutationLog::new(
        target.app,
        target.scope,
        file_path.display().to_string(),
        MutationAction::Disable,
        target.name.clone(),
        backup_path.map(|p| p.display().to_string()),
    );
    let _ = changelog::append(log);

    Ok(())
}

/// Reinserta la entrada en el config real (con el `Value` exacto que
/// tenía antes de deshabilitarse) y SOLO si eso tuvo éxito la quita del
/// sidecar.
///
/// ORDEN DELIBERADO, simétrico a `disable`: primero el config real
/// (`upsert_raw_value`, con su propio backup/atomic write), luego el
/// sidecar. Si el proceso muere entre medio, la entrada queda
/// duplicada (en el config real y todavía en el sidecar) en vez de
/// perdida — y una duplicación es fácil de detectar/resolver a mano
/// (o reintentando `enable`, que es idempotente sobre el config real
/// vía upsert), mientras que perder la única copia de una entrada
/// deshabilitada sería mucho peor.
pub fn enable(target: &McpTarget) -> Result<(), WriteError> {
    enable_with_sidecar(&disabled_file()?, target)
}

fn enable_with_sidecar(sidecar_path: &Path, target: &McpTarget) -> Result<(), WriteError> {
    let mut entries = read_all_at(sidecar_path)?;

    let key = sidecar_key(target);
    let entry_value = entries
        .get(&key)
        .cloned()
        .ok_or_else(|| WriteError::TargetNotFound {
            message: format!(
                "no se encontró '{}' entre los MCPs deshabilitados",
                target.name
            ),
        })?;

    let entry: DisabledEntry =
        serde_json::from_value(entry_value).map_err(|e| WriteError::Validation {
            path: sidecar_path.display().to_string(),
            message: e.to_string(),
        })?;

    let backup_path = mutations::upsert_raw_value(target, entry.config)?;

    // Solo quitamos del sidecar una vez que la entrada ya está a salvo
    // en el config real.
    entries.remove(&key);
    write_all_at(sidecar_path, &entries)?;

    let file_path = mutations::resolve_target_path(target)?;
    let log = crate::changelog::MutationLog::new(
        target.app,
        target.scope,
        file_path.display().to_string(),
        MutationAction::Enable,
        target.name.clone(),
        backup_path.map(|p| p.display().to_string()),
    );
    let _ = changelog::append(log);

    Ok(())
}

/// Lista todas las entradas deshabilitadas, junto con su clave de
/// sidecar, para que `get_inventory` las muestre con `status: disabled`.
pub fn list_disabled() -> Result<Vec<(String, DisabledEntry)>, WriteError> {
    let entries = read_all_at(&disabled_file()?)?;

    let mut result = Vec::new();
    for (key, value) in entries {
        if let Ok(entry) = serde_json::from_value::<DisabledEntry>(value) {
            result.push((key, entry));
        }
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn sidecar_key_is_stable_and_distinguishes_scope_and_project() {
        let user_target = McpTarget {
            app: AppId::ClaudeCode,
            scope: Scope::User,
            project_path: None,
            name: "hibob".to_string(),
        };
        let project_target = McpTarget {
            app: AppId::ClaudeCode,
            scope: Scope::Project,
            project_path: Some("/repo/a".to_string()),
            name: "hibob".to_string(),
        };

        assert_ne!(sidecar_key(&user_target), sidecar_key(&project_target));
    }

    #[test]
    fn disabled_entry_round_trips_through_json() {
        let entry = DisabledEntry {
            app: AppId::ClaudeDesktop,
            scope: Scope::User,
            project_path: None,
            name: "context7".to_string(),
            config: json!({ "command": "npx", "args": ["-y", "foo"] }),
        };

        let value = serde_json::to_value(&entry).expect("serializa");
        let reparsed: DisabledEntry = serde_json::from_value(value).expect("reparsea");

        assert_eq!(reparsed.name, "context7");
        assert_eq!(reparsed.config["command"], "npx");
    }

    /// Round-trip completo disable -> enable sobre un `.mcp.json` de
    /// proyecto en un tempdir (scope Project rutea a `<project>/.mcp.json`,
    /// así que no tocamos ningún archivo real del usuario) y un sidecar
    /// también en tempdir.
    #[test]
    fn disable_then_enable_round_trips_to_original_value() {
        let project_dir = tempfile::tempdir().expect("tempdir proyecto");
        let mcp_json = project_dir.path().join(".mcp.json");
        std::fs::write(
            &mcp_json,
            r#"{
                "mcpServers": {
                    "local-tool": {
                        "type": "stdio",
                        "command": "./bin/local-tool",
                        "env": { "TOKEN": "shh" }
                    }
                }
            }"#,
        )
        .expect("setup mcp.json");

        let sidecar_dir = tempfile::tempdir().expect("tempdir sidecar");
        let sidecar_path = sidecar_dir.path().join("disabled.json");

        let target = McpTarget {
            app: AppId::ClaudeCode,
            scope: Scope::Project,
            project_path: Some(project_dir.path().display().to_string()),
            name: "local-tool".to_string(),
        };

        let original_value = mutations::read_entry_value(&target).expect("leer original");

        disable_with_sidecar(&sidecar_path, &target).expect("disable");

        // Tras disable, la entrada ya no está en el .mcp.json.
        let after_disable: Value =
            serde_json::from_str(&std::fs::read_to_string(&mcp_json).unwrap()).unwrap();
        assert!(after_disable["mcpServers"].get("local-tool").is_none());

        enable_with_sidecar(&sidecar_path, &target).expect("enable");

        let restored_value = mutations::read_entry_value(&target).expect("leer restaurado");
        assert_eq!(restored_value, original_value);
    }

    /// Verifica el orden config-primero-sidecar-después de `disable`: al
    /// terminar `disable_with_sidecar`, la entrada ya no debe estar en el
    /// config real Y debe estar en el sidecar (nunca en ambos a la vez,
    /// que sería la duplicación que este orden evita).
    #[test]
    fn disable_removes_from_config_before_being_visible_in_sidecar_never_both() {
        let project_dir = tempfile::tempdir().expect("tempdir proyecto");
        let mcp_json = project_dir.path().join(".mcp.json");
        std::fs::write(
            &mcp_json,
            r#"{"mcpServers": {"local-tool": {"type": "stdio", "command": "./bin/x"}}}"#,
        )
        .expect("setup mcp.json");

        let sidecar_dir = tempfile::tempdir().expect("tempdir sidecar");
        let sidecar_path = sidecar_dir.path().join("disabled.json");

        let target = McpTarget {
            app: AppId::ClaudeCode,
            scope: Scope::Project,
            project_path: Some(project_dir.path().display().to_string()),
            name: "local-tool".to_string(),
        };

        disable_with_sidecar(&sidecar_path, &target).expect("disable");

        let config_after: Value =
            serde_json::from_str(&std::fs::read_to_string(&mcp_json).unwrap()).unwrap();
        let in_config = config_after["mcpServers"].get("local-tool").is_some();

        let sidecar_entries = read_all_at(&sidecar_path).expect("leer sidecar");
        let in_sidecar = sidecar_entries.contains_key(&sidecar_key(&target));

        assert!(!in_config, "la entrada debe salir del config real");
        assert!(in_sidecar, "la entrada debe terminar en el sidecar");
    }

    /// Simétrico para `enable`: al terminar, la entrada debe estar en el
    /// config real y haber sido quitada del sidecar.
    #[test]
    fn enable_adds_to_config_before_removing_from_sidecar_never_neither() {
        let project_dir = tempfile::tempdir().expect("tempdir proyecto");
        let mcp_json = project_dir.path().join(".mcp.json");
        std::fs::write(&mcp_json, r#"{"mcpServers": {}}"#).expect("setup mcp.json");

        let sidecar_dir = tempfile::tempdir().expect("tempdir sidecar");
        let sidecar_path = sidecar_dir.path().join("disabled.json");

        let target = McpTarget {
            app: AppId::ClaudeCode,
            scope: Scope::Project,
            project_path: Some(project_dir.path().display().to_string()),
            name: "local-tool".to_string(),
        };

        let entry = DisabledEntry {
            app: target.app,
            scope: target.scope,
            project_path: target.project_path.clone(),
            name: target.name.clone(),
            config: json!({ "type": "stdio", "command": "./bin/x" }),
        };
        let mut entries = Map::new();
        entries.insert(sidecar_key(&target), serde_json::to_value(&entry).unwrap());
        write_all_at(&sidecar_path, &entries).expect("setup sidecar");

        enable_with_sidecar(&sidecar_path, &target).expect("enable");

        let config_after: Value =
            serde_json::from_str(&std::fs::read_to_string(&mcp_json).unwrap()).unwrap();
        assert!(config_after["mcpServers"].get("local-tool").is_some());

        let sidecar_entries = read_all_at(&sidecar_path).expect("leer sidecar");
        assert!(!sidecar_entries.contains_key(&sidecar_key(&target)));
    }
}
