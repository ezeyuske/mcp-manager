use std::path::PathBuf;

use serde_json::Value;

use crate::domain::{
    AppId, AppInfo, ClaudeCodeFile, ClaudeCodeProject, McpInstallation, McpServerConfig, Scope,
};
use crate::error::AdapterError;

use super::AppAdapter;

pub struct ClaudeCodeAdapter;

const LABEL: &str = "Claude Code";

impl ClaudeCodeAdapter {
    /// `~/.claude.json`, igual en los tres OS (vive en el home del usuario,
    /// no en el directorio de config del sistema).
    fn config_path() -> Option<PathBuf> {
        dirs::home_dir().map(|home| home.join(".claude.json"))
    }

    fn read_file(path: &PathBuf) -> Result<ClaudeCodeFile, AdapterError> {
        let raw = std::fs::read_to_string(path).map_err(|source| AdapterError::Io {
            path: path.display().to_string(),
            source,
        })?;

        serde_json::from_str::<ClaudeCodeFile>(&raw).map_err(|e| AdapterError::InvalidJson {
            path: path.display().to_string(),
            message: e.to_string(),
        })
    }
}

impl AppAdapter for ClaudeCodeAdapter {
    fn id(&self) -> AppId {
        AppId::ClaudeCode
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
                    "No se pudo resolver el directorio home del usuario".to_string(),
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
                    Some("No se encontró ~/.claude.json".to_string())
                },
                error: None,
            },
        }
    }

    fn read(&self) -> Result<Vec<McpInstallation>, AdapterError> {
        let Some(path) = Self::config_path() else {
            return Ok(Vec::new());
        };

        if !path.exists() {
            return Ok(Vec::new());
        }

        let file = Self::read_file(&path)?;
        Ok(installations_from_file(&file, &path.display().to_string()))
    }
}

/// Lógica pura de conversión archivo -> instalaciones, separada de la
/// resolución de paths/I-O para poder testearla con fixtures en memoria.
///
/// Nota: las entradas de scope proyecto que viven en
/// `projects[path].mcpServers` de `~/.claude.json` reportan `config_path`
/// apuntando a `~/.claude.json` (que es de donde efectivamente se leen
/// hoy). Fase 3 solo agrega *nuevas* entradas de proyecto a `.mcp.json`
/// standalone; las legacy siguen viviendo y leyéndose desde acá.
fn installations_from_file(file: &ClaudeCodeFile, config_path: &str) -> Vec<McpInstallation> {
    let mut installations = Vec::new();

    // Scope user: mcpServers top-level.
    for (name, value) in file.mcp_servers.iter() {
        installations.push(parse_installation(
            name,
            value,
            Scope::User,
            None,
            config_path,
        ));
    }

    // Scope proyecto: projects[<path>].mcpServers.
    for (project_path, project_value) in file.projects.iter() {
        let project: Option<ClaudeCodeProject> = serde_json::from_value(project_value.clone()).ok();

        let Some(project) = project else {
            // Un proyecto con forma inesperada no debe tumbar el resto
            // del inventario: lo salteamos.
            continue;
        };

        for (name, value) in project.mcp_servers.iter() {
            installations.push(parse_installation(
                name,
                value,
                Scope::Project,
                Some(project_path.clone()),
                config_path,
            ));
        }
    }

    installations
}

fn parse_installation(
    name: &str,
    value: &Value,
    scope: Scope,
    project_path: Option<String>,
    config_path: &str,
) -> McpInstallation {
    let parsed: Option<McpServerConfig> = serde_json::from_value(value.clone()).ok();

    let Some(cfg) = parsed else {
        return McpInstallation {
            name: name.to_string(),
            app: AppId::ClaudeCode,
            scope,
            project_path,
            transport: crate::domain::TransportKind::Unknown,
            command: None,
            args: Vec::new(),
            url: None,
            env_keys: Vec::new(),
            status: crate::domain::McpStatus::Unknown,
            config_path: config_path.to_string(),
            enabled: true,
            vault_keys: Vec::new(),
            builtin: false,
        };
    };

    super::build_installation(
        name,
        AppId::ClaudeCode,
        scope,
        project_path,
        cfg,
        config_path.to_string(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::TransportKind;

    const FIXTURE: &str = r#"
    {
      "numStartups": 25,
      "installMethod": "native",
      "tipsHistory": {
        "memory-command": 16,
        "theme-command": 22
      },
      "mcpServers": {
        "hibob": {
          "type": "stdio",
          "command": "node",
          "args": ["/Users/ezequiel/mcps/hibob-mcp/src/index.js"]
        },
        "workspacemcp": {
          "type": "stdio",
          "command": "uvx",
          "args": ["workspace-mcp"],
          "env": {
            "GOOGLE_OAUTH_CLIENT_ID": "id-value",
            "GOOGLE_OAUTH_CLIENT_SECRET": "secret-value"
          }
        }
      },
      "projects": {
        "/Users/ezequiel/www/conexa/notion-guest-remover": {
          "allowedTools": [],
          "mcpContextUris": [],
          "enabledMcpjsonServers": [],
          "disabledMcpjsonServers": [],
          "hasTrustDialogAccepted": true,
          "mcpServers": {
            "local-tool": {
              "type": "stdio",
              "command": "./bin/local-tool",
              "env": {
                "TOKEN": "shh"
              }
            }
          }
        },
        "/Users/ezequiel/www/conexa/other-repo": {
          "allowedTools": [],
          "hasTrustDialogAccepted": true
        }
      }
    }
    "#;

    #[test]
    fn parses_user_and_project_scope_counts() {
        let file: ClaudeCodeFile = serde_json::from_str(FIXTURE).expect("fixture parsea");
        let installations = installations_from_file(&file, "/tmp/.claude.json");

        // 2 user-scope (hibob, workspacemcp) + 1 project-scope (local-tool).
        assert_eq!(installations.len(), 3);

        let user_count = installations
            .iter()
            .filter(|i| i.scope == Scope::User)
            .count();
        let project_count = installations
            .iter()
            .filter(|i| i.scope == Scope::Project)
            .count();

        assert_eq!(user_count, 2);
        assert_eq!(project_count, 1);
    }

    #[test]
    fn project_scope_entry_carries_project_path() {
        let file: ClaudeCodeFile = serde_json::from_str(FIXTURE).expect("fixture parsea");
        let installations = installations_from_file(&file, "/tmp/.claude.json");

        let local_tool = installations
            .iter()
            .find(|i| i.name == "local-tool")
            .expect("local-tool debe estar presente");

        assert_eq!(local_tool.scope, Scope::Project);
        assert_eq!(
            local_tool.project_path.as_deref(),
            Some("/Users/ezequiel/www/conexa/notion-guest-remover")
        );
        assert_eq!(local_tool.transport, TransportKind::Stdio);
        assert_eq!(local_tool.env_keys, vec!["TOKEN".to_string()]);
    }

    #[test]
    fn project_without_mcp_servers_key_contributes_no_installations() {
        let file: ClaudeCodeFile = serde_json::from_str(FIXTURE).expect("fixture parsea");
        let installations = installations_from_file(&file, "/tmp/.claude.json");

        assert!(installations
            .iter()
            .all(|i| i.project_path.as_deref() != Some("/Users/ezequiel/www/conexa/other-repo")));
    }

    #[test]
    fn env_values_never_leak_only_keys_do() {
        let file: ClaudeCodeFile = serde_json::from_str(FIXTURE).expect("fixture parsea");
        let installations = installations_from_file(&file, "/tmp/.claude.json");

        let workspacemcp = installations
            .iter()
            .find(|i| i.name == "workspacemcp")
            .expect("workspacemcp debe estar presente");

        assert_eq!(
            workspacemcp.env_keys,
            vec![
                "GOOGLE_OAUTH_CLIENT_ID".to_string(),
                "GOOGLE_OAUTH_CLIENT_SECRET".to_string()
            ]
        );

        let serialized = serde_json::to_string(&installations).expect("debe serializar");
        assert!(!serialized.contains("id-value"));
        assert!(!serialized.contains("secret-value"));
    }
}
