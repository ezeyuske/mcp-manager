use std::path::PathBuf;

use serde_json::Value;

use crate::domain::{
    AppId, AppInfo, ClaudeDesktopFile, McpInstallation, McpServerConfig, Scope, TransportKind,
};
use crate::error::AdapterError;

use super::AppAdapter;

pub struct ClaudeDesktopAdapter;

const LABEL: &str = "Claude Desktop";

impl ClaudeDesktopAdapter {
    /// Resuelve el path del config de Claude Desktop para este OS.
    /// Claude Desktop no existe en Linux: en ese caso devolvemos `None`
    /// en vez de inventar un path.
    fn config_path() -> Option<PathBuf> {
        if cfg!(target_os = "linux") {
            return None;
        }

        // macOS: ~/Library/Application Support/Claude/claude_desktop_config.json
        // Windows: %APPDATA%/Claude/claude_desktop_config.json
        // `dirs::config_dir()` resuelve a "Application Support" en macOS
        // y a `%APPDATA%` en Windows, respetando la convención de cada OS.
        let config_dir = dirs::config_dir()?;
        Some(config_dir.join("Claude").join("claude_desktop_config.json"))
    }

    fn read_file(path: &PathBuf) -> Result<ClaudeDesktopFile, AdapterError> {
        let raw = std::fs::read_to_string(path).map_err(|source| AdapterError::Io {
            path: path.display().to_string(),
            source,
        })?;

        serde_json::from_str::<ClaudeDesktopFile>(&raw).map_err(|e| AdapterError::InvalidJson {
            path: path.display().to_string(),
            message: e.to_string(),
        })
    }
}

impl AppAdapter for ClaudeDesktopAdapter {
    fn id(&self) -> AppId {
        AppId::ClaudeDesktop
    }

    fn label(&self) -> &'static str {
        LABEL
    }

    fn detect(&self) -> AppInfo {
        match Self::config_path() {
            None => AppInfo {
                id: self.id(),
                label: self.label().to_string(),
                installed: false,
                config_path: None,
                not_installed_reason: Some(
                    "Claude Desktop no está disponible en Linux".to_string(),
                ),
                error: None,
            },
            Some(path) => AppInfo {
                id: self.id(),
                label: self.label().to_string(),
                installed: path.exists(),
                config_path: Some(path.display().to_string()),
                not_installed_reason: if path.exists() {
                    None
                } else {
                    Some("No se encontró claude_desktop_config.json".to_string())
                },
                error: None,
            },
        }
    }

    fn read(&self) -> Result<Vec<McpInstallation>, AdapterError> {
        let Some(path) = Self::config_path() else {
            // No aplica a este OS: sin instalaciones, sin error.
            return Ok(Vec::new());
        };

        if !path.exists() {
            // App instalada pero sin config todavía (o sin MCPs): no es error.
            return Ok(Vec::new());
        }

        let file = Self::read_file(&path)?;
        Ok(installations_from_file(&file, &path.display().to_string()))
    }
}

/// Lógica pura de conversión archivo -> instalaciones, separada de la
/// resolución de paths/I-O para poder testearla con fixtures en memoria.
fn installations_from_file(file: &ClaudeDesktopFile, config_path: &str) -> Vec<McpInstallation> {
    file.mcp_servers
        .iter()
        .map(|(name, value)| parse_installation(name, value, config_path))
        .collect()
}

/// Convierte una entrada cruda de `mcpServers` en un `McpInstallation`
/// para el frontend. Si el valor no matchea la forma esperada de
/// `McpServerConfig`, se degrada a transporte/estado "unknown" en vez
/// de tumbar el resto del inventario.
fn parse_installation(name: &str, value: &Value, config_path: &str) -> McpInstallation {
    let parsed: Option<McpServerConfig> = serde_json::from_value(value.clone()).ok();

    let Some(cfg) = parsed else {
        return McpInstallation {
            name: name.to_string(),
            app: AppId::ClaudeDesktop,
            scope: Scope::User,
            project_path: None,
            transport: TransportKind::Unknown,
            command: None,
            args: Vec::new(),
            url: None,
            env_keys: Vec::new(),
            status: crate::domain::McpStatus::Unknown,
            config_path: config_path.to_string(),
            enabled: true,
        };
    };

    super::build_installation(
        name,
        AppId::ClaudeDesktop,
        Scope::User,
        None,
        cfg,
        config_path.to_string(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::McpStatus;

    const FIXTURE: &str = r#"
    {
      "coworkUserFilesPath": "/Users/ezequiel/Claude",
      "mcpServers": {
        "context7": {
          "command": "npx",
          "args": ["-y", "@upstash/context7-mcp@latest"]
        },
        "mcp-obsidian": {
          "command": "uv",
          "args": ["--directory", "/Users/ezequiel/mcps/mcp-obsidian", "run", "mcp-obsidian"],
          "env": {
            "OBSIDIAN_API_KEY": "super-secret-value",
            "OBSIDIAN_HOST": "127.0.0.1"
          }
        }
      },
      "preferences": {
        "quickEntryShortcut": "off",
        "sidebarMode": "chat"
      }
    }
    "#;

    #[test]
    fn parses_two_installations_with_user_scope() {
        let file: ClaudeDesktopFile = serde_json::from_str(FIXTURE).expect("fixture parsea");
        let installations = installations_from_file(&file, "/tmp/claude_desktop_config.json");

        assert_eq!(installations.len(), 2);
        assert!(installations.iter().all(|i| i.scope == Scope::User));
        assert!(installations.iter().all(|i| i.app == AppId::ClaudeDesktop));
        assert!(installations.iter().all(|i| i.project_path.is_none()));
        assert!(installations
            .iter()
            .all(|i| i.config_path == "/tmp/claude_desktop_config.json"));
        assert!(installations.iter().all(|i| i.enabled));
    }

    #[test]
    fn entry_without_type_infers_stdio_transport() {
        let file: ClaudeDesktopFile = serde_json::from_str(FIXTURE).expect("fixture parsea");
        let installations = installations_from_file(&file, "/tmp/claude_desktop_config.json");

        let context7 = installations
            .iter()
            .find(|i| i.name == "context7")
            .expect("context7 debe estar presente");

        assert_eq!(context7.transport, TransportKind::Stdio);
    }

    #[test]
    fn env_keys_are_present_but_never_values() {
        let file: ClaudeDesktopFile = serde_json::from_str(FIXTURE).expect("fixture parsea");
        let installations = installations_from_file(&file, "/tmp/claude_desktop_config.json");

        let obsidian = installations
            .iter()
            .find(|i| i.name == "mcp-obsidian")
            .expect("mcp-obsidian debe estar presente");

        assert_eq!(
            obsidian.env_keys,
            vec!["OBSIDIAN_API_KEY".to_string(), "OBSIDIAN_HOST".to_string()]
        );
        assert_ne!(obsidian.status, McpStatus::Unknown);
    }
}
