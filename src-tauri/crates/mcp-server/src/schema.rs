//! Esquemas de entrada de las tools, con tipos limpios y descriptivos para
//! el LLM. Se convierten a los tipos de dominio de `mcp-core` (que la GUI
//! también usa). Mantener esta capa fina: NADA de lógica de escritura acá.

use std::collections::BTreeMap;

use rmcp::schemars::{self, JsonSchema};
use serde::Deserialize;
use serde_json::{Map, Value};

use mcp_core::domain::{AppId, McpServerConfig, Scope};
use mcp_core::mutations::McpTarget;
use mcp_core::skills::{SkillInput, SkillTarget};

#[derive(Debug, Clone, Copy, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum AppArg {
    ClaudeDesktop,
    ClaudeCode,
}

impl From<AppArg> for AppId {
    fn from(a: AppArg) -> Self {
        match a {
            AppArg::ClaudeDesktop => AppId::ClaudeDesktop,
            AppArg::ClaudeCode => AppId::ClaudeCode,
        }
    }
}

#[derive(Debug, Clone, Copy, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum ScopeArg {
    User,
    Project,
}

impl From<ScopeArg> for Scope {
    fn from(s: ScopeArg) -> Self {
        match s {
            ScopeArg::User => Scope::User,
            ScopeArg::Project => Scope::Project,
        }
    }
}

/// Identifica una entrada de MCP.
#[derive(Debug, Clone, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct TargetArgs {
    /// Cliente: `claude-desktop` o `claude-code`.
    pub app: AppArg,
    /// Alcance: `user` o `project`.
    pub scope: ScopeArg,
    /// Ruta absoluta del proyecto (requerida cuando `scope` es `project`).
    #[serde(default)]
    pub project_path: Option<String>,
    /// Nombre de la entrada del MCP.
    pub name: String,
}

impl From<TargetArgs> for McpTarget {
    fn from(t: TargetArgs) -> Self {
        McpTarget {
            app: t.app.into(),
            scope: t.scope.into(),
            project_path: t.project_path,
            name: t.name,
        }
    }
}

/// Crea o edita un MCP.
#[derive(Debug, Clone, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct UpsertMcpArgs {
    pub app: AppArg,
    pub scope: ScopeArg,
    #[serde(default)]
    pub project_path: Option<String>,
    pub name: String,
    /// Comando ejecutable (transporte stdio).
    #[serde(default)]
    pub command: Option<String>,
    /// Argumentos del comando.
    #[serde(default)]
    pub args: Vec<String>,
    /// Variables de entorno (nombre → valor). NO usar para secretos:
    /// los secretos se gestionan aparte en el vault del keychain.
    #[serde(default)]
    pub env: BTreeMap<String, String>,
    /// URL del server (transportes `sse`/`http`).
    #[serde(default)]
    pub url: Option<String>,
    /// Tipo de transporte explícito: `stdio`, `sse` o `http`.
    #[serde(default, rename = "type")]
    pub transport_type: Option<String>,
}

impl UpsertMcpArgs {
    pub fn split(self) -> (McpTarget, McpServerConfig) {
        let target = McpTarget {
            app: self.app.into(),
            scope: self.scope.into(),
            project_path: self.project_path,
            name: self.name,
        };
        let env: Map<String, Value> = self
            .env
            .into_iter()
            .map(|(k, v)| (k, Value::String(v)))
            .collect();
        let config = McpServerConfig {
            r#type: self.transport_type,
            command: self.command,
            args: self.args,
            env,
            url: self.url,
            extra: Map::new(),
        };
        (target, config)
    }
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct SetEnabledArgs {
    #[serde(flatten)]
    pub target: TargetArgs,
    pub enabled: bool,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct DuplicateArgs {
    #[serde(flatten)]
    pub target: TargetArgs,
    /// Nuevo nombre para la copia.
    pub new_name: String,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct CopyArgs {
    /// Entrada origen.
    pub source: TargetArgs,
    /// Cliente destino.
    pub dest_app: AppArg,
    /// Alcance destino.
    pub dest_scope: ScopeArg,
    /// Ruta del proyecto destino (si `destScope` es `project`).
    #[serde(default)]
    pub dest_project_path: Option<String>,
}

/// Identifica una skill.
#[derive(Debug, Clone, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct SkillTargetArgs {
    pub scope: ScopeArg,
    #[serde(default)]
    pub project_path: Option<String>,
    pub name: String,
}

impl From<SkillTargetArgs> for SkillTarget {
    fn from(t: SkillTargetArgs) -> Self {
        SkillTarget {
            scope: t.scope.into(),
            project_path: t.project_path,
            name: t.name,
        }
    }
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct SetSkillEnabledArgs {
    #[serde(flatten)]
    pub target: SkillTargetArgs,
    pub enabled: bool,
}

/// Crea o edita una skill (`SKILL.md`).
#[derive(Debug, Clone, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct UpsertSkillArgs {
    pub scope: ScopeArg,
    #[serde(default)]
    pub project_path: Option<String>,
    /// Nombre (carpeta) de la skill. Sin separadores ni `..`.
    pub name: String,
    /// Descripción de qué hace y cuándo usarla (no vacía).
    pub description: String,
    #[serde(default)]
    pub version: Option<String>,
    /// Cuerpo markdown tras el frontmatter. Si se omite al editar, se
    /// preserva el body existente.
    #[serde(default)]
    pub body: String,
}

impl From<UpsertSkillArgs> for SkillInput {
    fn from(a: UpsertSkillArgs) -> Self {
        SkillInput {
            scope: a.scope.into(),
            project_path: a.project_path,
            name: a.name,
            description: a.description,
            version: a.version,
            body: a.body,
        }
    }
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ProjectArgs {
    /// Ruta absoluta del proyecto.
    pub path: String,
}
