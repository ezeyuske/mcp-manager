pub mod claude_code;
pub mod claude_desktop;

use crate::domain::{
    AppId, AppInfo, McpInstallation, McpServerConfig, McpStatus, Scope, TransportKind,
};
use crate::error::AdapterError;

/// Contrato común para cualquier app cuya config de MCPs este proyecto
/// sabe leer (y, en fases futuras, escribir).
///
/// Cada adapter resuelve sus propios paths por OS (con el crate `dirs`,
/// nunca hardcodeados) y decide si la app está instalada en este sistema.
pub trait AppAdapter {
    fn id(&self) -> crate::domain::AppId;
    fn label(&self) -> &'static str;

    /// Detecta si la app está instalada/soportada en este OS y, de ser
    /// así, dónde vive su config. No falla: si la app no aplica a este
    /// OS (p. ej. Claude Desktop en Linux) devuelve `installed: false`
    /// con una razón clara, nunca un path inventado.
    fn detect(&self) -> AppInfo;

    /// Lee las instalaciones de MCP de esta app. Un archivo inexistente
    /// no es un error (app instalada pero sin MCPs configurados todavía);
    /// un archivo corrupto sí lo es, y debe reportarse acotado a esta app.
    fn read(&self) -> Result<Vec<McpInstallation>, AdapterError>;
}

pub fn all_adapters() -> Vec<Box<dyn AppAdapter>> {
    vec![
        Box::new(claude_desktop::ClaudeDesktopAdapter),
        Box::new(claude_code::ClaudeCodeAdapter),
    ]
}

/// Infiere el transporte de una entrada de server: si trae `type`
/// explícito lo usamos; si no, pero hay `command`, asumimos stdio
/// (hoy es el único caso real sin `type` explícito). Sin `type` ni
/// `command` no podemos inferir nada confiable.
fn infer_transport(cfg: &McpServerConfig) -> TransportKind {
    match cfg.r#type.as_deref() {
        Some("stdio") => TransportKind::Stdio,
        Some("sse") => TransportKind::Sse,
        Some("http") => TransportKind::Http,
        Some(_) => TransportKind::Unknown,
        None if cfg.command.is_some() => TransportKind::Stdio,
        None => TransportKind::Unknown,
    }
}

/// Resuelve el estado de una instalación según su transporte:
/// - stdio: valida que el comando exista (en PATH, o como path absoluto).
/// - sse/http: remoto, no lo chequeamos acá => Ok.
/// - unknown: no podemos afirmar nada => Unknown.
fn resolve_status(transport: TransportKind, command: Option<&str>) -> McpStatus {
    match transport {
        TransportKind::Stdio => match command {
            Some(cmd) => {
                let found = if std::path::Path::new(cmd).is_absolute() {
                    std::path::Path::new(cmd).exists()
                } else {
                    which::which(cmd).is_ok()
                };
                if found {
                    McpStatus::Ok
                } else {
                    McpStatus::CommandNotFound
                }
            }
            None => McpStatus::CommandNotFound,
        },
        TransportKind::Sse | TransportKind::Http => McpStatus::Ok,
        TransportKind::Unknown => McpStatus::Unknown,
    }
}

/// Construye el DTO `McpInstallation` a partir de una entrada de server
/// ya parseada. Compartido entre adapters para no duplicar la lógica de
/// inferencia de transporte/estado y, sobre todo, para garantizar que
/// `env` nunca cruce al frontend salvo como lista de claves.
pub(crate) fn build_installation(
    name: &str,
    app: AppId,
    scope: Scope,
    project_path: Option<String>,
    cfg: McpServerConfig,
) -> McpInstallation {
    let transport = infer_transport(&cfg);
    let status = resolve_status(transport, cfg.command.as_deref());

    let mut env_keys: Vec<String> = cfg.env.keys().cloned().collect();
    env_keys.sort();

    McpInstallation {
        name: name.to_string(),
        app,
        scope,
        project_path,
        transport,
        command: cfg.command,
        args: cfg.args,
        url: cfg.url,
        env_keys,
        status,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn cfg_from(value: serde_json::Value) -> McpServerConfig {
        serde_json::from_value(value).expect("fixture de McpServerConfig debe parsear")
    }

    #[test]
    fn desktop_style_entry_without_type_infers_stdio() {
        let cfg = cfg_from(json!({
            "command": "npx",
            "args": ["-y", "@upstash/context7-mcp@latest"]
        }));

        assert_eq!(infer_transport(&cfg), TransportKind::Stdio);
    }

    #[test]
    fn explicit_type_is_respected() {
        let sse = cfg_from(json!({ "type": "sse", "url": "https://example.com/mcp" }));
        let http = cfg_from(json!({ "type": "http", "url": "https://example.com/mcp" }));
        let stdio = cfg_from(json!({ "type": "stdio", "command": "node" }));

        assert_eq!(infer_transport(&sse), TransportKind::Sse);
        assert_eq!(infer_transport(&http), TransportKind::Http);
        assert_eq!(infer_transport(&stdio), TransportKind::Stdio);
    }

    #[test]
    fn entry_without_type_nor_command_is_unknown() {
        let cfg = cfg_from(json!({ "url": "https://example.com/mcp" }));
        assert_eq!(infer_transport(&cfg), TransportKind::Unknown);
    }

    #[test]
    fn remote_transports_are_ok_without_checking_reachability() {
        assert_eq!(resolve_status(TransportKind::Sse, None), McpStatus::Ok);
        assert_eq!(resolve_status(TransportKind::Http, None), McpStatus::Ok);
    }

    #[test]
    fn unknown_transport_yields_unknown_status() {
        assert_eq!(
            resolve_status(TransportKind::Unknown, None),
            McpStatus::Unknown
        );
    }

    #[test]
    fn stdio_with_command_not_in_path_is_command_not_found() {
        let status = resolve_status(
            TransportKind::Stdio,
            Some("this-binary-should-not-exist-anywhere-xyz"),
        );
        assert_eq!(status, McpStatus::CommandNotFound);
    }

    #[test]
    fn build_installation_never_exposes_env_values_only_keys() {
        let cfg = cfg_from(json!({
            "command": "uv",
            "env": {
                "OBSIDIAN_API_KEY": "super-secret-value",
                "OBSIDIAN_HOST": "127.0.0.1"
            }
        }));

        let installation =
            build_installation("mcp-obsidian", AppId::ClaudeDesktop, Scope::User, None, cfg);

        assert_eq!(
            installation.env_keys,
            vec!["OBSIDIAN_API_KEY".to_string(), "OBSIDIAN_HOST".to_string()]
        );

        // La serialización a JSON (lo que ve el frontend) tampoco debe
        // contener ningún valor de env.
        let serialized = serde_json::to_string(&installation).expect("debe serializar");
        assert!(!serialized.contains("super-secret-value"));
        assert!(!serialized.contains("127.0.0.1"));
    }

    #[test]
    fn build_installation_sorts_env_keys() {
        let cfg = cfg_from(json!({
            "command": "node",
            "env": { "Z_VAR": "1", "A_VAR": "2" }
        }));

        let installation = build_installation("srv", AppId::ClaudeCode, Scope::User, None, cfg);

        assert_eq!(
            installation.env_keys,
            vec!["A_VAR".to_string(), "Z_VAR".to_string()]
        );
    }
}
