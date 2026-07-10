use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::changelog::{self, MutationAction, MutationLog};
use crate::domain::{AppId, ClaudeCodeFile, ClaudeDesktopFile, McpJsonFile, McpServerConfig, Scope};
use crate::error::WriteError;
use crate::safe_write;

/// Identifica unívocamente una entrada de MCP en algún config, tal como
/// la referencia el frontend. `project_path` solo aplica (y es
/// obligatorio en la práctica) cuando `scope == Scope::Project`.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct McpTarget {
    pub app: AppId,
    pub scope: Scope,
    pub project_path: Option<String>,
    pub name: String,
}

/// A qué modelo de archivo pertenece el destino resuelto, para saber
/// cómo cargarlo/mutarlo/guardarlo preservando sus claves ajenas.
enum TargetFile {
    /// `claude_desktop_config.json` — modelo `ClaudeDesktopFile`.
    ClaudeDesktop(PathBuf),
    /// `~/.claude.json`, `mcpServers` top-level — modelo `ClaudeCodeFile`.
    ClaudeCodeUser(PathBuf),
    /// `<project_path>/.mcp.json` — modelo `McpJsonFile`.
    McpJson(PathBuf),
}

impl TargetFile {
    fn path(&self) -> &Path {
        match self {
            TargetFile::ClaudeDesktop(p) => p,
            TargetFile::ClaudeCodeUser(p) => p,
            TargetFile::McpJson(p) => p,
        }
    }

    /// Slug estable para el subdirectorio de backups de este archivo.
    fn backup_slug(&self) -> &'static str {
        match self {
            TargetFile::ClaudeDesktop(_) => "claude-desktop",
            TargetFile::ClaudeCodeUser(_) => "claude-code",
            TargetFile::McpJson(_) => "mcp-json",
        }
    }
}

// Resuelve el archivo destino para un `McpTarget`, según la matriz
// app/scope descrita en el plan de Fase 3:
//
// - claude-desktop / user    -> <config_dir>/Claude/claude_desktop_config.json
// - claude-code    / user    -> ~/.claude.json (mcpServers top-level)
// - *              / project -> <project_path>/.mcp.json
fn resolve_target_file(target: &McpTarget) -> Result<TargetFile, WriteError> {
    match target.scope {
        Scope::Project => {
            let project_path = target.project_path.as_ref().ok_or_else(|| {
                WriteError::TargetNotFound {
                    message: "scope Project requiere projectPath".to_string(),
                }
            })?;
            Ok(TargetFile::McpJson(
                PathBuf::from(project_path).join(".mcp.json"),
            ))
        }
        Scope::User => match target.app {
            AppId::ClaudeDesktop => {
                let config_dir = dirs::config_dir().ok_or_else(|| WriteError::NotSupported {
                    message: "no se pudo resolver el directorio de config del sistema"
                        .to_string(),
                })?;
                Ok(TargetFile::ClaudeDesktop(
                    config_dir.join("Claude").join("claude_desktop_config.json"),
                ))
            }
            AppId::ClaudeCode => {
                let home = dirs::home_dir().ok_or_else(|| WriteError::NotSupported {
                    message: "no se pudo resolver el directorio home del usuario".to_string(),
                })?;
                Ok(TargetFile::ClaudeCodeUser(home.join(".claude.json")))
            }
        },
    }
}

// ---------------------------------------------------------------------
// Carga/guardado por modelo, preservando SIEMPRE las claves ajenas a
// nivel archivo. Cada función de mutación opera sobre el
// `Map<String, Value>` de `mcpServers` y nada más.
// ---------------------------------------------------------------------

fn load_mcp_servers(file: &TargetFile) -> Result<(Value, Map<String, Value>), WriteError> {
    let path = file.path();

    if !path.exists() {
        // Archivo inexistente: partimos de una estructura mínima vacía,
        // del modelo correspondiente, para no inventar claves ajenas.
        let empty = match file {
            TargetFile::ClaudeDesktop(_) => serde_json::to_value(ClaudeDesktopFile {
                mcp_servers: Map::new(),
                extra: Map::new(),
            }),
            TargetFile::ClaudeCodeUser(_) => serde_json::to_value(ClaudeCodeFile {
                mcp_servers: Map::new(),
                projects: Map::new(),
                extra: Map::new(),
            }),
            TargetFile::McpJson(_) => serde_json::to_value(McpJsonFile {
                mcp_servers: Map::new(),
                extra: Map::new(),
            }),
        }
        .map_err(|e| WriteError::Serialize {
            message: e.to_string(),
        })?;

        return Ok((empty, Map::new()));
    }

    let raw = std::fs::read_to_string(path).map_err(|source| WriteError::Io {
        path: path.display().to_string(),
        source,
    })?;

    let mut root: Value = serde_json::from_str(&raw).map_err(|e| WriteError::Validation {
        path: path.display().to_string(),
        message: e.to_string(),
    })?;

    if !root.is_object() {
        return Err(WriteError::Validation {
            path: path.display().to_string(),
            message: "se esperaba un objeto JSON en la raíz del archivo".to_string(),
        });
    }

    let mcp_servers = root
        .as_object_mut()
        .and_then(|obj| obj.get("mcpServers"))
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();

    Ok((root, mcp_servers))
}

/// Reinyecta el `mcpServers` mutado en el `Value` raíz (preservando todo
/// lo demás byte a byte) y lo persiste con el pipeline seguro.
fn save_mcp_servers(
    file: &TargetFile,
    mut root: Value,
    mcp_servers: Map<String, Value>,
) -> Result<Option<PathBuf>, WriteError> {
    let obj = root.as_object_mut().ok_or_else(|| WriteError::Serialize {
        message: "la raíz del archivo dejó de ser un objeto JSON".to_string(),
    })?;

    if mcp_servers.is_empty() {
        obj.remove("mcpServers");
    } else {
        obj.insert("mcpServers".to_string(), Value::Object(mcp_servers));
    }

    safe_write::write_json(file.path(), &root, file.backup_slug())
}

fn log_mutation(
    target: &McpTarget,
    file: &TargetFile,
    action: MutationAction,
    backup_path: Option<PathBuf>,
) {
    let entry = MutationLog::new(
        target.app,
        target.scope,
        file.path().display().to_string(),
        action,
        target.name.clone(),
        backup_path.map(|p| p.display().to_string()),
    );

    // El changelog es "best effort": si falla (p. ej. no se pudo crear
    // ~/.mcp-manager), no debe revertir ni enmascarar una mutación que
    // ya se escribió con éxito en el config real.
    let _ = changelog::append(entry);
}

/// Adapta un `Value` de entrada existente sumando/actualizando SOLO los
/// campos conocidos de `McpServerConfig` (type/command/args/env/url),
/// preservando cualquier otro campo que ya tuviera la entrada (su
/// `extra`, aunque el frontend no lo conozca).
fn merge_entry(existing: &Value, incoming: &McpServerConfig) -> Result<Value, WriteError> {
    let mut merged = match existing {
        Value::Object(map) => map.clone(),
        _ => Map::new(),
    };

    match &incoming.r#type {
        Some(t) => {
            merged.insert("type".to_string(), Value::String(t.clone()));
        }
        None => {
            merged.remove("type");
        }
    }

    match &incoming.command {
        Some(c) => {
            merged.insert("command".to_string(), Value::String(c.clone()));
        }
        None => {
            merged.remove("command");
        }
    }

    if incoming.args.is_empty() {
        merged.remove("args");
    } else {
        merged.insert(
            "args".to_string(),
            Value::Array(incoming.args.iter().cloned().map(Value::String).collect()),
        );
    }

    // NOTA DE SEGURIDAD: a diferencia de args/command/url/type, el
    // frontend NUNCA recibe los valores de `env` existentes (son
    // secretos y no se exponen fuera del proceso Rust). Si tratáramos
    // `incoming.env` vacío como "el usuario quiere borrar env", un
    // simple edit de otro campo (p. ej. renombrar el comando) desde un
    // form que no gestiona env mandaría env=={} y borraría los
    // secretos del usuario silenciosamente. Por eso, en `merge_entry`
    // (que SOLO corre en EDIT; en ADD se serializa `config` directo y
    // no pasa por acá): env vacío entrante => preservamos el env que ya
    // tenía la entrada, sin tocar la clave. Env con entries entrante
    // => reemplaza como los demás campos, porque ahí sí es una edición
    // explícita gestionada por el frontend.
    if !incoming.env.is_empty() {
        merged.insert("env".to_string(), Value::Object(incoming.env.clone()));
    }

    match &incoming.url {
        Some(u) => {
            merged.insert("url".to_string(), Value::String(u.clone()));
        }
        None => {
            merged.remove("url");
        }
    }

    // Campos desconocidos que trae `incoming` (su propio `extra`) también
    // se aplican, para no perder ediciones futuras del frontend.
    for (k, v) in incoming.extra.iter() {
        merged.insert(k.clone(), v.clone());
    }

    Ok(Value::Object(merged))
}

/// ADD (la clave no existe) o EDIT (la clave existe: merge quirúrgico a
/// nivel entrada, preservando los campos desconocidos de la entrada
/// existente).
pub fn upsert(target: &McpTarget, config: McpServerConfig) -> Result<Option<PathBuf>, WriteError> {
    let file = resolve_target_file(target)?;
    upsert_into(&file, target, config)
}

fn upsert_into(
    file: &TargetFile,
    target: &McpTarget,
    config: McpServerConfig,
) -> Result<Option<PathBuf>, WriteError> {
    let (root, mut mcp_servers) = load_mcp_servers(file)?;

    let is_edit = mcp_servers.contains_key(&target.name);

    let new_value = if let Some(existing) = mcp_servers.get(&target.name) {
        merge_entry(existing, &config)?
    } else {
        serde_json::to_value(&config).map_err(|e| WriteError::Serialize {
            message: e.to_string(),
        })?
    };

    mcp_servers.insert(target.name.clone(), new_value);

    let backup_path = save_mcp_servers(file, root, mcp_servers)?;

    log_mutation(
        target,
        file,
        if is_edit {
            MutationAction::Edit
        } else {
            MutationAction::Add
        },
        backup_path.clone(),
    );

    Ok(backup_path)
}

/// Quita la clave `target.name` del `mcpServers` correspondiente.
pub fn delete(target: &McpTarget) -> Result<Option<PathBuf>, WriteError> {
    let file = resolve_target_file(target)?;
    let (root, mut mcp_servers) = load_mcp_servers(&file)?;

    if mcp_servers.remove(&target.name).is_none() {
        return Err(WriteError::TargetNotFound {
            message: format!(
                "no se encontró la entrada '{}' en {}",
                target.name,
                file.path().display()
            ),
        });
    }

    let backup_path = save_mcp_servers(&file, root, mcp_servers)?;
    log_mutation(target, &file, MutationAction::Delete, backup_path.clone());

    Ok(backup_path)
}

/// Copia el `Value` de `target.name` bajo `new_name`, en el mismo archivo.
pub fn duplicate(target: &McpTarget, new_name: &str) -> Result<Option<PathBuf>, WriteError> {
    let file = resolve_target_file(target)?;
    let (root, mut mcp_servers) = load_mcp_servers(&file)?;

    let existing = mcp_servers
        .get(&target.name)
        .cloned()
        .ok_or_else(|| WriteError::TargetNotFound {
            message: format!(
                "no se encontró la entrada '{}' en {}",
                target.name,
                file.path().display()
            ),
        })?;

    mcp_servers.insert(new_name.to_string(), existing);

    let backup_path = save_mcp_servers(&file, root, mcp_servers)?;

    let mut duplicated_target = target.clone();
    duplicated_target.name = new_name.to_string();
    log_mutation(
        &duplicated_target,
        &file,
        MutationAction::Duplicate,
        backup_path.clone(),
    );

    Ok(backup_path)
}

/// Adapta el `Value` de una entrada al formato del `dest_app`, o falla
/// con un error claro si el transporte de origen no es soportado en el
/// destino (p. ej. http/sse hacia Claude Desktop).
fn adapt_entry_for(dest_app: AppId, entry: &Value) -> Result<Value, WriteError> {
    let mut map = match entry {
        Value::Object(m) => m.clone(),
        _ => {
            return Err(WriteError::Validation {
                path: "<entry>".to_string(),
                message: "la entrada de origen no es un objeto JSON".to_string(),
            })
        }
    };

    let transport_type = map.get("type").and_then(Value::as_str).map(str::to_string);

    match dest_app {
        AppId::ClaudeDesktop => {
            match transport_type.as_deref() {
                Some("http") | Some("sse") => {
                    return Err(WriteError::NotSupported {
                        message:
                            "Claude Desktop solo soporta MCPs stdio; el origen es http/sse"
                                .to_string(),
                    });
                }
                _ => {}
            }
            // Claude Desktop no usa la clave "type": la quitamos.
            map.remove("type");
        }
        AppId::ClaudeCode => {
            // Si es (o infiere ser) stdio, aseguramos "type":"stdio"
            // explícito, que es la convención de Claude Code.
            let is_stdio_like = matches!(transport_type.as_deref(), Some("stdio") | None)
                && map.contains_key("command");
            if is_stdio_like {
                map.insert("type".to_string(), Value::String("stdio".to_string()));
            }
        }
    }

    Ok(Value::Object(map))
}

/// Lee el `Value` de `source`, lo adapta al formato de `dest_app`, y lo
/// inserta en el destino resuelto por `dest_app`/`dest_scope`/`dest_project_path`
/// (vía la misma lógica de `upsert`, para heredar backup/validación).
pub fn copy(
    source: &McpTarget,
    dest_app: AppId,
    dest_scope: Scope,
    dest_project_path: Option<String>,
) -> Result<Option<PathBuf>, WriteError> {
    let source_file = resolve_target_file(source)?;
    let (_, source_servers) = load_mcp_servers(&source_file)?;

    let source_value =
        source_servers
            .get(&source.name)
            .cloned()
            .ok_or_else(|| WriteError::TargetNotFound {
                message: format!(
                    "no se encontró la entrada '{}' en {}",
                    source.name,
                    source_file.path().display()
                ),
            })?;

    let adapted_value = adapt_entry_for(dest_app, &source_value)?;

    let dest_target = McpTarget {
        app: dest_app,
        scope: dest_scope,
        project_path: dest_project_path,
        name: source.name.clone(),
    };
    let dest_file = resolve_target_file(&dest_target)?;
    let (dest_root, mut dest_servers) = load_mcp_servers(&dest_file)?;

    dest_servers.insert(dest_target.name.clone(), adapted_value);

    let backup_path = save_mcp_servers(&dest_file, dest_root, dest_servers)?;
    log_mutation(&dest_target, &dest_file, MutationAction::Copy, backup_path.clone());

    Ok(backup_path)
}

/// Lee el `Value` crudo de una entrada (usado por `disabled.rs` para
/// mover una entrada al sidecar sin perder ningún campo desconocido).
pub fn read_entry_value(target: &McpTarget) -> Result<Value, WriteError> {
    let file = resolve_target_file(target)?;
    let (_, mcp_servers) = load_mcp_servers(&file)?;

    mcp_servers
        .get(&target.name)
        .cloned()
        .ok_or_else(|| WriteError::TargetNotFound {
            message: format!(
                "no se encontró la entrada '{}' en {}",
                target.name,
                file.path().display()
            ),
        })
}

/// Inserta un `Value` crudo bajo `target.name` (usado por `disabled.rs`
/// para reinsertar una entrada al habilitarla, preservando exactamente
/// el `Value` que se guardó en el sidecar).
pub fn upsert_raw_value(target: &McpTarget, value: Value) -> Result<Option<PathBuf>, WriteError> {
    let file = resolve_target_file(target)?;
    let (root, mut mcp_servers) = load_mcp_servers(&file)?;

    mcp_servers.insert(target.name.clone(), value);

    save_mcp_servers(&file, root, mcp_servers)
}

/// Path del archivo destino, expuesto para que `disabled.rs`/`commands.rs`
/// puedan reportarlo sin duplicar la lógica de resolución.
pub fn resolve_target_path(target: &McpTarget) -> Result<PathBuf, WriteError> {
    Ok(resolve_target_file(target)?.path().to_path_buf())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn target(name: &str) -> McpTarget {
        McpTarget {
            app: AppId::ClaudeDesktop,
            scope: Scope::User,
            project_path: None,
            name: name.to_string(),
        }
    }

    fn desktop_fixture() -> &'static str {
        r#"
        {
          "coworkUserFilesPath": "/Users/ezequiel/Claude",
          "mcpServers": {
            "context7": {
              "command": "npx",
              "args": ["-y", "@upstash/context7-mcp@latest"]
            }
          },
          "preferences": {
            "quickEntryShortcut": "off",
            "sidebarMode": "chat"
          }
        }
        "#
    }

    #[test]
    fn upsert_add_preserves_foreign_top_level_keys() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("claude_desktop_config.json");
        std::fs::write(&path, desktop_fixture()).expect("setup");

        let file = TargetFile::ClaudeDesktop(path.clone());
        let (root, mut mcp_servers) = load_mcp_servers(&file).expect("load");

        let cfg: McpServerConfig = serde_json::from_value(json!({
            "command": "uv",
            "args": ["run", "mcp-obsidian"]
        }))
        .unwrap();
        mcp_servers.insert("mcp-obsidian".to_string(), serde_json::to_value(&cfg).unwrap());

        save_mcp_servers(&file, root, mcp_servers).expect("save");

        let written: Value =
            serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        assert_eq!(
            written["coworkUserFilesPath"],
            "/Users/ezequiel/Claude"
        );
        assert_eq!(written["preferences"]["sidebarMode"], "chat");
        assert!(written["mcpServers"]["context7"].is_object());
        assert!(written["mcpServers"]["mcp-obsidian"].is_object());
    }

    #[test]
    fn upsert_edit_preserves_unknown_entry_fields() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("claude_desktop_config.json");
        std::fs::write(
            &path,
            r#"{
                "mcpServers": {
                    "context7": {
                        "command": "npx",
                        "args": ["-y", "old"],
                        "disabled": true,
                        "someFutureField": "keep-me"
                    }
                }
            }"#,
        )
        .expect("setup");

        let t = target("context7");
        let new_cfg: McpServerConfig = serde_json::from_value(json!({
            "command": "npx",
            "args": ["-y", "new"]
        }))
        .unwrap();

        let file = TargetFile::ClaudeDesktop(path.clone());
        upsert_into(&file, &t, new_cfg).expect("upsert edit");

        let written: Value =
            serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        let entry = &written["mcpServers"]["context7"];
        assert_eq!(entry["args"][1], "new");
        assert_eq!(entry["disabled"], true);
        assert_eq!(entry["someFutureField"], "keep-me");
    }

    /// El frontend nunca recibe los valores de `env` existentes; un edit
    /// que llega con `env` vacío NO debe borrar el env real ya guardado.
    #[test]
    fn upsert_edit_with_empty_incoming_env_preserves_existing_env() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("claude_desktop_config.json");
        std::fs::write(
            &path,
            r#"{
                "mcpServers": {
                    "context7": {
                        "command": "npx",
                        "args": ["-y", "old"],
                        "env": { "API_KEY": "super-secret" }
                    }
                }
            }"#,
        )
        .expect("setup");

        let t = target("context7");
        // El form del frontend no gestiona env: llega vacío.
        let new_cfg: McpServerConfig = serde_json::from_value(json!({
            "command": "npx",
            "args": ["-y", "new"]
        }))
        .unwrap();

        let file = TargetFile::ClaudeDesktop(path.clone());
        upsert_into(&file, &t, new_cfg).expect("upsert edit");

        let written: Value =
            serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        let entry = &written["mcpServers"]["context7"];
        assert_eq!(entry["args"][1], "new");
        assert_eq!(entry["env"]["API_KEY"], "super-secret");
    }

    /// Si el edit trae `env` con entries, sí reemplaza (edición explícita
    /// gestionada por el frontend).
    #[test]
    fn upsert_edit_with_nonempty_incoming_env_replaces_existing_env() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("claude_desktop_config.json");
        std::fs::write(
            &path,
            r#"{
                "mcpServers": {
                    "context7": {
                        "command": "npx",
                        "args": ["-y", "old"],
                        "env": { "API_KEY": "super-secret" }
                    }
                }
            }"#,
        )
        .expect("setup");

        let t = target("context7");
        let new_cfg: McpServerConfig = serde_json::from_value(json!({
            "command": "npx",
            "args": ["-y", "old"],
            "env": { "NEW_KEY": "new-value" }
        }))
        .unwrap();

        let file = TargetFile::ClaudeDesktop(path.clone());
        upsert_into(&file, &t, new_cfg).expect("upsert edit");

        let written: Value =
            serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        let entry = &written["mcpServers"]["context7"];
        assert_eq!(entry["env"]["NEW_KEY"], "new-value");
        assert!(entry["env"].get("API_KEY").is_none());
    }

    #[test]
    fn delete_removes_only_target_key() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("claude_desktop_config.json");
        std::fs::write(
            &path,
            r#"{"mcpServers": {"a": {"command": "x"}, "b": {"command": "y"}}}"#,
        )
        .expect("setup");

        let file = TargetFile::ClaudeDesktop(path.clone());
        let (root, mut mcp_servers) = load_mcp_servers(&file).expect("load");
        mcp_servers.remove("a");
        save_mcp_servers(&file, root, mcp_servers).expect("save");

        let written: Value =
            serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        assert!(written["mcpServers"].get("a").is_none());
        assert!(written["mcpServers"].get("b").is_some());
    }

    #[test]
    fn upsert_on_claude_code_user_file_preserves_foreign_keys_and_top_level_order() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join(".claude.json");
        std::fs::write(
            &path,
            r#"{
                "numStartups": 25,
                "installMethod": "native",
                "mcpServers": {
                    "hibob": { "type": "stdio", "command": "node" }
                },
                "projects": {
                    "/repo/a": { "hasTrustDialogAccepted": true }
                }
            }"#,
        )
        .expect("setup");

        let original_raw = std::fs::read_to_string(&path).unwrap();
        let original: Value = serde_json::from_str(&original_raw).unwrap();
        let original_keys: Vec<String> = original
            .as_object()
            .unwrap()
            .keys()
            .cloned()
            .collect();

        let t = McpTarget {
            app: AppId::ClaudeCode,
            scope: Scope::User,
            project_path: None,
            name: "new-server".to_string(),
        };
        let cfg: McpServerConfig = serde_json::from_value(json!({
            "type": "stdio",
            "command": "npx",
            "args": ["-y", "foo"]
        }))
        .unwrap();

        let file = TargetFile::ClaudeCodeUser(path.clone());
        upsert_into(&file, &t, cfg).expect("upsert add");

        let written_raw = std::fs::read_to_string(&path).unwrap();
        let written: Value = serde_json::from_str(&written_raw).unwrap();

        // Claves ajenas a nivel raíz sobreviven.
        assert_eq!(written["numStartups"], 25);
        assert_eq!(written["installMethod"], "native");
        assert_eq!(
            written["projects"]["/repo/a"]["hasTrustDialogAccepted"],
            true
        );

        // La nueva entrada está presente y la vieja también.
        assert!(written["mcpServers"]["hibob"].is_object());
        assert!(written["mcpServers"]["new-server"].is_object());

        // Con `preserve_order`, el orden de las claves top-level se
        // mantiene (solo cambia el contenido de mcpServers, no su
        // posición ni la de sus hermanas).
        let written_keys: Vec<String> = written.as_object().unwrap().keys().cloned().collect();
        assert_eq!(written_keys, original_keys);
    }

    #[test]
    fn adapt_entry_for_claude_desktop_strips_type_for_stdio() {
        let entry = json!({ "type": "stdio", "command": "node", "args": [] });
        let adapted = adapt_entry_for(AppId::ClaudeDesktop, &entry).expect("debe adaptar");
        assert!(adapted.get("type").is_none());
        assert_eq!(adapted["command"], "node");
    }

    #[test]
    fn adapt_entry_for_claude_desktop_rejects_http() {
        let entry = json!({ "type": "http", "url": "https://example.com/mcp" });
        let result = adapt_entry_for(AppId::ClaudeDesktop, &entry);
        assert!(result.is_err());
    }

    #[test]
    fn adapt_entry_for_claude_code_adds_stdio_type() {
        let entry = json!({ "command": "npx", "args": ["-y", "foo"] });
        let adapted = adapt_entry_for(AppId::ClaudeCode, &entry).expect("debe adaptar");
        assert_eq!(adapted["type"], "stdio");
    }

    /// `copy` end-to-end usando scope Project en ambos extremos (source y
    /// dest resuelven a `.mcp.json` en tempdirs distintos), para no tocar
    /// nada del filesystem real del usuario.
    #[test]
    fn copy_from_stdio_project_to_another_project_strips_nothing_relevant() {
        let source_dir = tempfile::tempdir().expect("tempdir source");
        std::fs::write(
            source_dir.path().join(".mcp.json"),
            r#"{"mcpServers": {"local-tool": {"type": "stdio", "command": "./bin/x"}}}"#,
        )
        .expect("setup source");

        let dest_dir = tempfile::tempdir().expect("tempdir dest");

        let source = McpTarget {
            app: AppId::ClaudeCode,
            scope: Scope::Project,
            project_path: Some(source_dir.path().display().to_string()),
            name: "local-tool".to_string(),
        };

        copy(
            &source,
            AppId::ClaudeCode,
            Scope::Project,
            Some(dest_dir.path().display().to_string()),
        )
        .expect("copy debe funcionar");

        let dest_written: Value = serde_json::from_str(
            &std::fs::read_to_string(dest_dir.path().join(".mcp.json")).unwrap(),
        )
        .unwrap();
        assert_eq!(dest_written["mcpServers"]["local-tool"]["type"], "stdio");
        assert_eq!(
            dest_written["mcpServers"]["local-tool"]["command"],
            "./bin/x"
        );
    }
}
