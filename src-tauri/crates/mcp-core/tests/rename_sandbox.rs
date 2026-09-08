//! E2E del rename de MCPs bajo el SANDBOX de desarrollo
//! (`MCP_MANAGER_CONFIG_ROOT`): ejercita el path público real
//! (`rename::preflight` + `rename::apply`) contra los dos modelos de
//! archivo, el sidecar de deshabilitados y los bindings del vault, sin
//! tocar ningún config real del usuario.
//!
//! Un solo `#[test]` en este binario de integración: setear la env var es
//! process-global, así que aislarlo en su propio proceso evita races con
//! el resto de la suite (mismo criterio que `env_editing_sandbox.rs`).

use mcp_core::domain::{AppId, Scope};
use mcp_core::mutations::McpTarget;
use mcp_core::rename::{self, Location};
use serde_json::Value;

fn read_json(path: &std::path::Path) -> Value {
    serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
}

#[test]
fn rename_mcp_under_sandbox() {
    let root = tempfile::tempdir().expect("tempdir");
    std::env::set_var(mcp_core::paths::CONFIG_ROOT_ENV, root.path());
    assert!(
        mcp_core::paths::sandbox_active(),
        "el sandbox debería estar activo con la env var seteada"
    );

    let desktop_path = root
        .path()
        .join("config")
        .join("Claude")
        .join("claude_desktop_config.json");
    std::fs::create_dir_all(desktop_path.parent().unwrap()).unwrap();
    std::fs::write(
        &desktop_path,
        r#"{
            "coworkUserFilesPath": "/keep/me",
            "mcpServers": {
                "context7": {
                    "command": "npx",
                    "args": ["-y", "ctx"],
                    "someFutureField": "keep",
                    "env": { "TOKEN": "s3cr3t", "OTHER": "plain" }
                },
                "taken": { "command": "true" }
            },
            "preferences": { "sidebarMode": "chat" }
        }"#,
    )
    .unwrap();

    let home = root.path().join("home");
    std::fs::create_dir_all(&home).unwrap();
    let claude_json = home.join(".claude.json");
    std::fs::write(
        &claude_json,
        r#"{
            "numStartups": 42,
            "projects": { "/repo/a": { "history": ["algo"] } },
            "mcpServers": {
                "context7": { "command": "npx", "args": ["-y", "ctx"] }
            }
        }"#,
    )
    .unwrap();

    let desktop = McpTarget {
        app: AppId::ClaudeDesktop,
        scope: Scope::User,
        project_path: None,
        name: "context7".to_string(),
    };
    let code = McpTarget {
        app: AppId::ClaudeCode,
        scope: Scope::User,
        project_path: None,
        name: "context7".to_string(),
    };

    // -----------------------------------------------------------------
    // 1. Un binding del vault apuntando al target viejo. Solo metadata:
    //    no se toca el keychain en ningún momento de este test.
    // -----------------------------------------------------------------
    let vault_path = mcp_core::paths::vault_file().unwrap();
    std::fs::write(
        &vault_path,
        serde_json::to_string_pretty(&serde_json::json!({
            "secrets": ["ctx7-token", "otro-secreto"],
            "bindings": [
                {
                    "target": { "app": "claude-desktop", "scope": "user", "projectPath": null, "name": "context7" },
                    "envKey": "TOKEN",
                    "secretName": "ctx7-token"
                },
                {
                    "target": { "app": "claude-desktop", "scope": "user", "projectPath": null, "name": "otro-mcp" },
                    "envKey": "TOKEN",
                    "secretName": "otro-secreto"
                }
            ]
        }))
        .unwrap(),
    )
    .unwrap();

    // -----------------------------------------------------------------
    // 2. Colisión: el preflight falla y NO escribe nada.
    // -----------------------------------------------------------------
    let before = std::fs::read_to_string(&desktop_path).unwrap();
    let err = rename::preflight(&desktop, "taken").expect_err("debería colisionar");
    assert!(
        err.to_string().contains("taken"),
        "el error debería nombrar el nombre tomado: {err}"
    );
    assert_eq!(
        std::fs::read_to_string(&desktop_path).unwrap(),
        before,
        "un preflight fallido no debe tocar el archivo"
    );

    // Nombres inválidos, también sin escribir.
    for bad in ["", "con espacio", "a/b", "..", " lead"] {
        assert!(
            rename::preflight(&desktop, bad).is_err(),
            "'{bad}' debería rechazarse"
        );
    }
    assert_eq!(std::fs::read_to_string(&desktop_path).unwrap(), before);

    // -----------------------------------------------------------------
    // 3. Rename en los dos archivos (el caso "todas las instalaciones").
    // -----------------------------------------------------------------
    for target in [&desktop, &code] {
        let location = rename::preflight(target, "ctx7").expect("preflight");
        assert_eq!(location, Location::Config);
        rename::apply(target, "ctx7", location).expect("apply");
    }

    let desktop_json = read_json(&desktop_path);
    let entry = &desktop_json["mcpServers"]["ctx7"];
    assert!(desktop_json["mcpServers"].get("context7").is_none());
    // El `Value` se movió entero: campos desconocidos y env intactos.
    assert_eq!(entry["someFutureField"], "keep");
    assert_eq!(entry["args"][1], "ctx");
    assert_eq!(entry["env"]["TOKEN"], "s3cr3t");
    assert_eq!(entry["env"]["OTHER"], "plain");
    // Claves ajenas del archivo, y la entrada vecina, intactas.
    assert_eq!(desktop_json["coworkUserFilesPath"], "/keep/me");
    assert_eq!(desktop_json["preferences"]["sidebarMode"], "chat");
    assert_eq!(desktop_json["mcpServers"]["taken"]["command"], "true");

    let code_json = read_json(&claude_json);
    assert!(code_json["mcpServers"].get("context7").is_none());
    assert_eq!(code_json["mcpServers"]["ctx7"]["args"][1], "ctx");
    // ~/.claude.json tiene mucho más que MCPs: nada de eso se toca.
    assert_eq!(code_json["numStartups"], 42);
    assert_eq!(code_json["projects"]["/repo/a"]["history"][0], "algo");

    // -----------------------------------------------------------------
    // 4. El binding del vault siguió al target; el de otro MCP no se tocó.
    // -----------------------------------------------------------------
    let vault_json = read_json(&vault_path);
    let bindings = vault_json["bindings"].as_array().unwrap();
    assert_eq!(bindings.len(), 2);
    assert_eq!(bindings[0]["target"]["name"], "ctx7");
    assert_eq!(bindings[0]["secretName"], "ctx7-token");
    assert_eq!(
        bindings[1]["target"]["name"], "otro-mcp",
        "el binding de otro MCP no debe moverse"
    );
    assert_eq!(
        vault_json["secrets"].as_array().unwrap().len(),
        2,
        "la lista de secretos no cambia en un rename"
    );

    // -----------------------------------------------------------------
    // 5. Backups: cada archivo tocado dejó el suyo, así que un rename
    //    parcial siempre es recuperable.
    // -----------------------------------------------------------------
    let backups = home.join(".mcp-manager").join("backups");
    assert!(backups.join("claude-desktop").read_dir().unwrap().count() > 0);
    assert!(backups.join("claude-code").read_dir().unwrap().count() > 0);

    // -----------------------------------------------------------------
    // 6. Rename de un MCP DESHABILITADO: la entrada vive en el sidecar,
    //    no en el config, y ahí es donde hay que re-keyearla.
    // -----------------------------------------------------------------
    let renamed_desktop = McpTarget {
        name: "ctx7".to_string(),
        ..desktop.clone()
    };
    mcp_core::disabled::disable(&renamed_desktop).expect("disable");

    let disabled_path = mcp_core::paths::disabled_file().unwrap();
    assert!(
        read_json(&desktop_path)["mcpServers"].get("ctx7").is_none(),
        "deshabilitar saca la entrada del config real"
    );

    let location = rename::preflight(&renamed_desktop, "ctx7-off").expect("preflight disabled");
    assert_eq!(location, Location::DisabledSidecar);
    rename::apply(&renamed_desktop, "ctx7-off", location).expect("apply disabled");

    let disabled_json = read_json(&disabled_path);
    let entry = disabled_json
        .as_object()
        .unwrap()
        .values()
        .next()
        .expect("una entrada deshabilitada");
    assert_eq!(entry["name"], "ctx7-off");
    // El config crudo guardado se preserva verbatim, env incluido.
    assert_eq!(entry["config"]["env"]["TOKEN"], "s3cr3t");
    assert_eq!(entry["config"]["someFutureField"], "keep");

    // Y al habilitarlo, vuelve al config real con el nombre nuevo.
    let off_target = McpTarget {
        name: "ctx7-off".to_string(),
        ..desktop.clone()
    };
    mcp_core::disabled::enable(&off_target).expect("enable");
    let desktop_json = read_json(&desktop_path);
    assert_eq!(
        desktop_json["mcpServers"]["ctx7-off"]["env"]["TOKEN"],
        "s3cr3t"
    );
    assert_eq!(desktop_json["coworkUserFilesPath"], "/keep/me");

    // -----------------------------------------------------------------
    // 7. El changelog registró los renames con "viejo → nuevo".
    // -----------------------------------------------------------------
    let logs = mcp_core::changelog::list().expect("changelog");
    let renames: Vec<_> = logs
        .iter()
        .filter(|l| l.action == mcp_core::changelog::MutationAction::Rename)
        .collect();
    assert_eq!(renames.len(), 3, "dos configs + una entrada deshabilitada");
    assert!(renames.iter().any(|l| l.mcp_name == "context7 → ctx7"));
    assert!(renames.iter().any(|l| l.mcp_name == "ctx7 → ctx7-off"));
}
