//! Orquestación multi-target del command `rename_mcp`, bajo el SANDBOX de
//! desarrollo (`MCP_MANAGER_CONFIG_ROOT`).
//!
//! Lo que se verifica acá y no en `mcp-core` es la garantía que el core no
//! puede dar por sí solo: el PREFLIGHT DE TODOS los targets antes de
//! escribir el primero. Un rename cruza varios archivos y por lo tanto no
//! puede ser atómico, así que esa fase previa es lo único que evita el
//! peor escenario realista (el nombre nuevo ya está tomado en el segundo
//! archivo y el primero queda renombrado a medias).
//!
//! Un solo `#[test]` por binario: `set_var` es process-global.

use mcp_core::domain::{AppId, Scope};
use mcp_core::mutations::McpTarget;
use mcp_manager_lib::commands::rename_mcp;

#[test]
fn rename_command_preflights_every_target_before_writing() {
    let root = tempfile::tempdir().expect("tempdir");
    std::env::set_var(mcp_core::paths::CONFIG_ROOT_ENV, root.path());
    assert!(mcp_core::paths::sandbox_active());

    // Claude Desktop: solo "context7".
    let desktop_path = root
        .path()
        .join("config")
        .join("Claude")
        .join("claude_desktop_config.json");
    std::fs::create_dir_all(desktop_path.parent().unwrap()).unwrap();
    std::fs::write(
        &desktop_path,
        r#"{"mcpServers":{"context7":{"command":"npx","args":["-y","ctx"]}}}"#,
    )
    .unwrap();

    // Claude Code: "context7" Y un "ctx7" que ya ocupa el nombre destino.
    let home = root.path().join("home");
    std::fs::create_dir_all(&home).unwrap();
    let claude_json = home.join(".claude.json");
    std::fs::write(
        &claude_json,
        r#"{"numStartups":7,"mcpServers":{"context7":{"command":"npx"},"ctx7":{"command":"otro"}}}"#,
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

    let desktop_before = std::fs::read_to_string(&desktop_path).unwrap();
    let code_before = std::fs::read_to_string(&claude_json).unwrap();

    // El target que colisiona es el SEGUNDO: sin preflight global, el
    // primero ya habría quedado renombrado cuando falla el segundo.
    let err = rename_mcp(vec![desktop.clone(), code.clone()], "ctx7".to_string())
        .expect_err("debería fallar por colisión en el segundo target");
    assert!(
        err.contains("ctx7"),
        "el error debería nombrar el conflicto: {err}"
    );

    assert_eq!(
        std::fs::read_to_string(&desktop_path).unwrap(),
        desktop_before,
        "el PRIMER archivo no debe haberse tocado pese a que su propio preflight pasaba"
    );
    assert_eq!(
        std::fs::read_to_string(&claude_json).unwrap(),
        code_before,
        "el segundo archivo tampoco"
    );

    // No se creó ningún backup, porque no hubo ninguna escritura.
    let backups = home.join(".mcp-manager").join("backups");
    assert!(
        !backups.exists() || backups.read_dir().unwrap().count() == 0,
        "un rename abortado en preflight no debería generar backups"
    );

    // Con un nombre libre, los dos targets se renombran y el reporte lo dice.
    let report = rename_mcp(vec![desktop, code], "context-seven".to_string()).expect("rename");
    assert_eq!(report.renamed.len(), 2);
    assert!(report.failed.is_empty());
    assert!(report.renamed.iter().all(|t| t.name == "context-seven"));

    let desktop_json: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&desktop_path).unwrap()).unwrap();
    assert_eq!(
        desktop_json["mcpServers"]["context-seven"]["args"][1],
        "ctx"
    );

    let code_json: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&claude_json).unwrap()).unwrap();
    assert_eq!(code_json["mcpServers"]["context-seven"]["command"], "npx");
    // La entrada vecina que ocupaba el nombre y las claves ajenas siguen ahí.
    assert_eq!(code_json["mcpServers"]["ctx7"]["command"], "otro");
    assert_eq!(code_json["numStartups"], 7);
}
