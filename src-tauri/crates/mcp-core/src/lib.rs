//! Núcleo libre de Tauri de mcp-manager.
//!
//! Contiene toda la lógica de dominio y de escritura segura de configs
//! (adapters, mutations, safe_write, vault, skills, ...). Es la ÚNICA
//! fuente de verdad de esa lógica: la reutilizan tanto el backend Tauri
//! (`mcp-manager`) como el servidor MCP standalone (`mcp-server`), para
//! no duplicar el código más crítico y peligroso del proyecto.

pub mod adapters;
pub mod builtin;
pub mod changelog;
pub mod disabled;
pub mod domain;
pub mod error;
pub mod inventory;
pub mod mutations;
pub mod paths;
pub mod projects;
pub mod rename;
pub mod safe_write;
pub mod skills;
pub mod vault;
