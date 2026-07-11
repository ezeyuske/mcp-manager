//! Servidor MCP standalone de mcp-manager (transporte stdio).
//!
//! Expone a un LLM el MISMO subconjunto seguro de operaciones que la app,
//! reutilizando `mcp-core` (única fuente de verdad de la escritura segura
//! de configs). Se distribuye como sidecar de la app de escritorio.
//!
//! Subconjunto SEGURO deliberado: NO se exponen operaciones destructivas
//! (delete de MCPs/skills, restore de backups) ni NADA del vault de
//! secretos (jamás `vault_reveal`). Lo que no está definido como `#[tool]`
//! acá, simplemente no existe para el LLM.
//!
//! IMPORTANTE: el transporte stdio es dueño de STDOUT. Nunca escribir a
//! stdout (ni `println!`): rompería el stream JSON-RPC. Todo diagnóstico
//! va a stderr.

mod schema;

use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::{CallToolResult, ContentBlock};
use rmcp::transport::io::stdio;
use rmcp::{tool, tool_router, ServiceExt};

use mcp_core::builtin::BUILTIN_NAME;
use mcp_core::domain::Scope;

use schema::{
    CopyArgs, DuplicateArgs, ProjectArgs, SetEnabledArgs, SetSkillEnabledArgs, UpsertMcpArgs,
    UpsertSkillArgs,
};

#[derive(Clone)]
struct McpManagerServer;

/// Serializa un valor a JSON pretty como contenido textual de la tool.
fn ok_json<T: serde::Serialize>(value: &T) -> CallToolResult {
    match serde_json::to_string_pretty(value) {
        Ok(s) => CallToolResult::success(vec![ContentBlock::text(s)]),
        Err(e) => CallToolResult::error(vec![ContentBlock::text(format!(
            "error serializando la respuesta: {e}"
        ))]),
    }
}

fn ok_msg(msg: impl Into<String>) -> CallToolResult {
    CallToolResult::success(vec![ContentBlock::text(msg.into())])
}

fn err_msg(msg: impl Into<String>) -> CallToolResult {
    CallToolResult::error(vec![ContentBlock::text(msg.into())])
}

/// Rechaza mutaciones directas sobre la entrada interna del built-in.
fn is_builtin(scope: Scope, name: &str) -> bool {
    scope == Scope::User && name == BUILTIN_NAME
}

#[tool_router(server_handler)]
impl McpManagerServer {
    #[tool(
        name = "list_inventory",
        description = "Lista el inventario unificado de MCPs de Claude Desktop y Claude Code (user y project), incluyendo estado, transporte y si están deshabilitados."
    )]
    async fn list_inventory(&self) -> CallToolResult {
        ok_json(&mcp_core::inventory::build())
    }

    #[tool(
        name = "list_skills",
        description = "Lista las skills instaladas (scope user y project), con su descripción y si están habilitadas."
    )]
    async fn list_skills(&self) -> CallToolResult {
        match mcp_core::skills::read_skills() {
            Ok(s) => ok_json(&s),
            Err(e) => err_msg(e.to_string()),
        }
    }

    #[tool(
        name = "list_projects",
        description = "Lista las rutas de proyectos registrados en mcp-manager."
    )]
    async fn list_projects(&self) -> CallToolResult {
        match mcp_core::projects::list() {
            Ok(p) => ok_json(&p),
            Err(e) => err_msg(e.to_string()),
        }
    }

    #[tool(
        name = "list_changelog",
        description = "Lista el historial de mutaciones hechas por mcp-manager (más reciente primero)."
    )]
    async fn list_changelog(&self) -> CallToolResult {
        match mcp_core::changelog::list() {
            Ok(c) => ok_json(&c),
            Err(e) => err_msg(e.to_string()),
        }
    }

    #[tool(
        name = "upsert_mcp",
        description = "Crea o edita una entrada de MCP en el cliente indicado. Escritura segura: backup + escritura atómica + merge que preserva claves ajenas."
    )]
    async fn upsert_mcp(&self, Parameters(args): Parameters<UpsertMcpArgs>) -> CallToolResult {
        let (target, config) = args.split();
        if is_builtin(target.scope, &target.name) {
            return err_msg(format!(
                "'{BUILTIN_NAME}' es la entrada interna de la app: se gobierna con su toggle, no con upsert"
            ));
        }
        match mcp_core::mutations::upsert(&target, config) {
            Ok(_) => ok_msg(format!("MCP '{}' guardado.", target.name)),
            Err(e) => err_msg(e.to_string()),
        }
    }

    #[tool(
        name = "set_mcp_enabled",
        description = "Habilita o deshabilita un MCP. Deshabilitar lo mueve a un sidecar recuperable sin borrarlo del disco."
    )]
    async fn set_mcp_enabled(
        &self,
        Parameters(args): Parameters<SetEnabledArgs>,
    ) -> CallToolResult {
        let target: mcp_core::mutations::McpTarget = args.target.into();
        if is_builtin(target.scope, &target.name) {
            return err_msg(format!(
                "'{BUILTIN_NAME}' es interno: usá el toggle del built-in en la app"
            ));
        }
        let result = if args.enabled {
            mcp_core::disabled::enable(&target)
        } else {
            mcp_core::disabled::disable(&target)
        };
        match result {
            Ok(_) => ok_msg(format!(
                "MCP '{}' {}.",
                target.name,
                if args.enabled { "habilitado" } else { "deshabilitado" }
            )),
            Err(e) => err_msg(e.to_string()),
        }
    }

    #[tool(
        name = "duplicate_mcp",
        description = "Duplica un MCP existente con un nombre nuevo en el mismo cliente/scope."
    )]
    async fn duplicate_mcp(&self, Parameters(args): Parameters<DuplicateArgs>) -> CallToolResult {
        let new_name = args.new_name.clone();
        let target: mcp_core::mutations::McpTarget = args.target.into();
        match mcp_core::mutations::duplicate(&target, &new_name) {
            Ok(_) => ok_msg(format!("MCP '{}' duplicado como '{}'.", target.name, new_name)),
            Err(e) => err_msg(e.to_string()),
        }
    }

    #[tool(
        name = "copy_mcp",
        description = "Copia un MCP de un cliente/scope a otro destino, preservando su configuración."
    )]
    async fn copy_mcp(&self, Parameters(args): Parameters<CopyArgs>) -> CallToolResult {
        let source: mcp_core::mutations::McpTarget = args.source.into();
        match mcp_core::mutations::copy(
            &source,
            args.dest_app.into(),
            args.dest_scope.into(),
            args.dest_project_path,
        ) {
            Ok(_) => ok_msg(format!("MCP '{}' copiado al destino.", source.name)),
            Err(e) => err_msg(e.to_string()),
        }
    }

    #[tool(
        name = "set_skill_enabled",
        description = "Habilita o deshabilita una skill. Deshabilitar la mueve a un sidecar recuperable, no la borra."
    )]
    async fn set_skill_enabled(
        &self,
        Parameters(args): Parameters<SetSkillEnabledArgs>,
    ) -> CallToolResult {
        let target: mcp_core::skills::SkillTarget = args.target.into();
        match mcp_core::skills::set_skill_enabled(&target, args.enabled) {
            Ok(_) => ok_msg(format!(
                "skill '{}' {}.",
                target.name,
                if args.enabled { "habilitada" } else { "deshabilitada" }
            )),
            Err(e) => err_msg(e.to_string()),
        }
    }

    #[tool(
        name = "upsert_skill",
        description = "Crea o edita una skill escribiendo su SKILL.md de forma segura (frontmatter YAML válido, escritura atómica, sin tocar otros archivos de la carpeta). Al editar sin body, preserva el body existente."
    )]
    async fn upsert_skill(&self, Parameters(args): Parameters<UpsertSkillArgs>) -> CallToolResult {
        let name = args.name.clone();
        let input: mcp_core::skills::SkillInput = args.into();
        match mcp_core::skills::upsert_skill(&input) {
            Ok(_) => ok_msg(format!("skill '{name}' guardada.")),
            Err(e) => err_msg(e.to_string()),
        }
    }

    #[tool(
        name = "register_project",
        description = "Registra una ruta de proyecto para que mcp-manager gestione sus MCPs/skills de scope project."
    )]
    async fn register_project(&self, Parameters(args): Parameters<ProjectArgs>) -> CallToolResult {
        match mcp_core::projects::register(&args.path) {
            Ok(_) => ok_msg(format!("proyecto '{}' registrado.", args.path)),
            Err(e) => err_msg(e.to_string()),
        }
    }

    #[tool(
        name = "unregister_project",
        description = "Quita una ruta de proyecto del registro de mcp-manager (no borra archivos del proyecto)."
    )]
    async fn unregister_project(
        &self,
        Parameters(args): Parameters<ProjectArgs>,
    ) -> CallToolResult {
        match mcp_core::projects::unregister(&args.path) {
            Ok(_) => ok_msg(format!("proyecto '{}' desregistrado.", args.path)),
            Err(e) => err_msg(e.to_string()),
        }
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let service = McpManagerServer.serve(stdio()).await?;
    service.waiting().await?;
    Ok(())
}
