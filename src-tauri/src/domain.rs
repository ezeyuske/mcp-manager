use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

// ---------------------------------------------------------------------
// Enums / DTOs expuestos al frontend
// ---------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum AppId {
    ClaudeDesktop,
    ClaudeCode,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Scope {
    User,
    Project,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TransportKind {
    Stdio,
    Sse,
    Http,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum McpStatus {
    Ok,
    CommandNotFound,
    Unknown,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct McpInstallation {
    pub name: String,
    pub app: AppId,
    pub scope: Scope,
    pub project_path: Option<String>,
    pub transport: TransportKind,
    pub command: Option<String>,
    pub args: Vec<String>,
    pub url: Option<String>,
    pub env_keys: Vec<String>,
    pub status: McpStatus,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppInfo {
    pub id: AppId,
    pub label: String,
    pub installed: bool,
    pub config_path: Option<String>,
    pub not_installed_reason: Option<String>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Inventory {
    pub apps: Vec<AppInfo>,
    pub installations: Vec<McpInstallation>,
}

// ---------------------------------------------------------------------
// Modelos de archivo — preservan claves ajenas para permitir, en fases
// futuras, un merge quirúrgico que no toque nada que este proyecto no
// administre. Nunca reescribir un archivo entero a partir de un modelo
// parcial: por eso cada struct lleva `#[serde(flatten)]` sobre un mapa.
// ---------------------------------------------------------------------

/// Entrada de servidor MCP, transport-agnóstica. Modela los campos
/// conocidos hoy (stdio, y a futuro sse/http) y preserva cualquier
/// campo desconocido en `extra`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct McpServerConfig {
    #[serde(rename = "type", default, skip_serializing_if = "Option::is_none")]
    pub r#type: Option<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub command: Option<String>,

    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub args: Vec<String>,

    #[serde(default, skip_serializing_if = "Map::is_empty")]
    pub env: Map<String, Value>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,

    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

/// `claude_desktop_config.json`
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ClaudeDesktopFile {
    #[serde(rename = "mcpServers", default, skip_serializing_if = "Map::is_empty")]
    pub mcp_servers: Map<String, Value>,

    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

/// Un proyecto dentro de `projects` en `~/.claude.json`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ClaudeCodeProject {
    #[serde(rename = "mcpServers", default, skip_serializing_if = "Map::is_empty")]
    pub mcp_servers: Map<String, Value>,

    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

/// `~/.claude.json`. Contiene MUCHO más que MCPs (historial, settings,
/// estado de proyectos) — este proyecto solo modela `mcpServers` (scope
/// user) y `projects[*].mcpServers` (scope proyecto). Todo lo demás
/// (incluidas las sub-keys ajenas de cada proyecto) vive en `extra` /
/// `ClaudeCodeProject::extra` y se preserva byte a byte en round-trip.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ClaudeCodeFile {
    #[serde(rename = "mcpServers", default, skip_serializing_if = "Map::is_empty")]
    pub mcp_servers: Map<String, Value>,

    #[serde(default, skip_serializing_if = "Map::is_empty")]
    pub projects: Map<String, Value>,

    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

#[cfg(test)]
mod tests {
    use super::*;

    const CLAUDE_DESKTOP_FIXTURE: &str = r#"
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

    const CLAUDE_CODE_FIXTURE: &str = r#"
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
        }
      }
    }
    "#;

    #[test]
    fn claude_desktop_file_parses_known_and_preserves_unknown_keys() {
        let file: ClaudeDesktopFile =
            serde_json::from_str(CLAUDE_DESKTOP_FIXTURE).expect("fixture debe parsear");

        assert_eq!(file.mcp_servers.len(), 2);
        assert_eq!(
            file.extra
                .get("coworkUserFilesPath")
                .and_then(Value::as_str),
            Some("/Users/ezequiel/Claude")
        );
        assert!(file.extra.get("preferences").is_some());
    }

    #[test]
    fn claude_desktop_file_round_trip_preserves_foreign_keys() {
        let file: ClaudeDesktopFile =
            serde_json::from_str(CLAUDE_DESKTOP_FIXTURE).expect("fixture debe parsear");

        let serialized = serde_json::to_string(&file).expect("debe serializar");
        let reparsed: ClaudeDesktopFile =
            serde_json::from_str(&serialized).expect("debe re-parsear");

        assert_eq!(file, reparsed);
        assert_eq!(
            reparsed
                .extra
                .get("coworkUserFilesPath")
                .and_then(Value::as_str),
            Some("/Users/ezequiel/Claude")
        );
        assert_eq!(
            reparsed
                .extra
                .get("preferences")
                .and_then(|v| v.get("sidebarMode"))
                .and_then(Value::as_str),
            Some("chat")
        );
        assert_eq!(reparsed.mcp_servers.len(), 2);
    }

    #[test]
    fn claude_code_file_round_trip_preserves_foreign_keys_including_project_subkeys() {
        let file: ClaudeCodeFile =
            serde_json::from_str(CLAUDE_CODE_FIXTURE).expect("fixture debe parsear");

        let serialized = serde_json::to_string(&file).expect("debe serializar");
        let reparsed: ClaudeCodeFile = serde_json::from_str(&serialized).expect("debe re-parsear");

        assert_eq!(file, reparsed);

        // Claves ajenas a nivel raíz.
        assert_eq!(
            reparsed.extra.get("numStartups").and_then(Value::as_i64),
            Some(25)
        );
        assert_eq!(
            reparsed.extra.get("installMethod").and_then(Value::as_str),
            Some("native")
        );
        assert_eq!(
            reparsed
                .extra
                .get("tipsHistory")
                .and_then(|v| v.get("theme-command"))
                .and_then(Value::as_i64),
            Some(22)
        );

        // Sub-keys ajenas dentro de un proyecto.
        let project = reparsed
            .projects
            .get("/Users/ezequiel/www/conexa/notion-guest-remover")
            .expect("el proyecto debe seguir presente");
        assert_eq!(
            project
                .get("hasTrustDialogAccepted")
                .and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(
            project
                .get("allowedTools")
                .and_then(Value::as_array)
                .map(Vec::len),
            Some(0)
        );
    }

    #[test]
    fn claude_code_project_parses_mcp_servers_and_preserves_extra() {
        let file: ClaudeCodeFile =
            serde_json::from_str(CLAUDE_CODE_FIXTURE).expect("fixture debe parsear");

        let project_value = file
            .projects
            .get("/Users/ezequiel/www/conexa/notion-guest-remover")
            .expect("proyecto presente");

        let project: ClaudeCodeProject =
            serde_json::from_value(project_value.clone()).expect("debe parsear el proyecto");

        assert_eq!(project.mcp_servers.len(), 1);
        assert!(project.extra.contains_key("hasTrustDialogAccepted"));
        assert!(project.extra.contains_key("allowedTools"));
    }

    #[test]
    fn mcp_server_config_preserves_unknown_field_in_extra() {
        let raw = r#"{
            "command": "npx",
            "args": ["-y", "foo"],
            "disabled": true
        }"#;

        let cfg: McpServerConfig = serde_json::from_str(raw).expect("debe parsear");

        assert_eq!(cfg.command.as_deref(), Some("npx"));
        assert_eq!(
            cfg.extra.get("disabled").and_then(Value::as_bool),
            Some(true)
        );

        // Round-trip: el campo desconocido sigue ahí.
        let serialized = serde_json::to_string(&cfg).expect("debe serializar");
        let reparsed: McpServerConfig = serde_json::from_str(&serialized).expect("debe re-parsear");
        assert_eq!(
            reparsed.extra.get("disabled").and_then(Value::as_bool),
            Some(true)
        );
    }
}
