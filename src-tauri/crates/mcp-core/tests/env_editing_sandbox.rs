//! E2E del path real de edición de env bajo el SANDBOX de desarrollo
//! (`MCP_MANAGER_CONFIG_ROOT`): ejercita la resolución de paths por OS
//! (`paths::config_dir` → `<root>/config/Claude/...`), la lectura on-demand
//! y la escritura quirúrgica con backup/atómica/validación, sin tocar
//! ningún config real del usuario.
//!
//! Un solo `#[test]` en este binario de integración: setear la env var es
//! process-global, así que aislarlo en su propio proceso evita races con
//! el resto de la suite.

use mcp_core::domain::{AppId, Scope};
use mcp_core::mutations::{self, McpTarget};
use serde_json::{Map, Value};

#[test]
fn read_and_surgically_edit_env_under_sandbox() {
    let root = tempfile::tempdir().expect("tempdir");
    std::env::set_var(mcp_core::paths::CONFIG_ROOT_ENV, root.path());
    assert!(
        mcp_core::paths::sandbox_active(),
        "el sandbox debería estar activo con la env var seteada"
    );

    // Claude Desktop / user resuelve a <root>/config/Claude/claude_desktop_config.json
    let config_path = root
        .path()
        .join("config")
        .join("Claude")
        .join("claude_desktop_config.json");
    std::fs::create_dir_all(config_path.parent().unwrap()).unwrap();
    std::fs::write(
        &config_path,
        r#"{
            "coworkUserFilesPath": "/keep/me",
            "mcpServers": {
                "context7": {
                    "command": "npx",
                    "args": ["-y", "ctx"],
                    "someFutureField": "keep",
                    "env": { "KEEP": "untouched", "EDIT_ME": "before" }
                }
            }
        }"#,
    )
    .unwrap();

    let target = McpTarget {
        app: AppId::ClaudeDesktop,
        scope: Scope::User,
        project_path: None,
        name: "context7".to_string(),
    };

    // 1. read_env_value on-demand.
    assert_eq!(
        mutations::read_env_value(&target, "EDIT_ME").unwrap(),
        "before"
    );
    assert!(mutations::read_env_value(&target, "NOPE").is_err());

    // 2. set_mcp_env quirúrgico: editar una, agregar otra, quitar KEEP.
    let mut upserts = Map::new();
    upserts.insert("EDIT_ME".to_string(), Value::String("after".to_string()));
    upserts.insert("NEW_KEY".to_string(), Value::String("added".to_string()));
    mutations::set_mcp_env(&target, upserts, vec!["KEEP".to_string()]).unwrap();

    let written: Value =
        serde_json::from_str(&std::fs::read_to_string(&config_path).unwrap()).unwrap();
    let entry = &written["mcpServers"]["context7"];

    // Cambios aplicados.
    assert_eq!(entry["env"]["EDIT_ME"], "after");
    assert_eq!(entry["env"]["NEW_KEY"], "added");
    assert!(entry["env"].get("KEEP").is_none());
    // Preservación de claves ajenas (entrada y top-level).
    assert_eq!(entry["someFutureField"], "keep");
    assert_eq!(entry["args"][1], "ctx");
    assert_eq!(written["coworkUserFilesPath"], "/keep/me");

    // 3. Se generó un backup bajo <root>/home/.mcp-manager/backups/.
    let backups = root.path().join("home").join(".mcp-manager").join("backups");
    assert!(
        backups.exists(),
        "debería existir el directorio de backups del sandbox"
    );
    let has_backup = std::fs::read_dir(&backups)
        .map(|rd| rd.filter_map(Result::ok).any(|_| true))
        .unwrap_or(false);
    assert!(has_backup, "set_mcp_env debería haber dejado un backup");

    std::env::remove_var(mcp_core::paths::CONFIG_ROOT_ENV);
}
