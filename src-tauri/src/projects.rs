use std::path::Path;

use crate::error::WriteError;
use crate::paths::projects_file;

fn read_all_at(path: &Path) -> Result<Vec<String>, WriteError> {
    if !path.exists() {
        return Ok(Vec::new());
    }

    let raw = std::fs::read_to_string(path).map_err(|source| WriteError::Io {
        path: path.display().to_string(),
        source,
    })?;

    if raw.trim().is_empty() {
        return Ok(Vec::new());
    }

    serde_json::from_str(&raw).map_err(|e| WriteError::Validation {
        path: path.display().to_string(),
        message: e.to_string(),
    })
}

/// Escritura directa (sin `safe_write`): `projects.json` es estado
/// INTERNO de mcp-manager (la lista de directorios de proyecto que el
/// usuario registró en esta app), no un config ajeno de terceros. Mismo
/// razonamiento que `changelog.rs` y `disabled.rs`: no amerita el
/// backup/atomic write reservado para configs de otras aplicaciones.
fn write_all_at(path: &Path, paths: &[String]) -> Result<(), WriteError> {
    let serialized = serde_json::to_string_pretty(paths).map_err(|e| WriteError::Serialize {
        message: e.to_string(),
    })?;

    std::fs::write(path, serialized).map_err(|source| WriteError::Io {
        path: path.display().to_string(),
        source,
    })
}

/// Registra un path de repo si todavía no está presente (idempotente).
fn register_at(path: &Path, project_path: &str) -> Result<(), WriteError> {
    let mut paths = read_all_at(path)?;
    if !paths.iter().any(|p| p == project_path) {
        paths.push(project_path.to_string());
    }
    write_all_at(path, &paths)
}

/// Registra un directorio de proyecto conocido en
/// `~/.mcp-manager/projects.json`.
pub fn register(project_path: &str) -> Result<(), WriteError> {
    register_at(&projects_file()?, project_path)
}

/// Lista los directorios de proyecto conocidos.
pub fn list() -> Result<Vec<String>, WriteError> {
    read_all_at(&projects_file()?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn register_is_idempotent() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("projects.json");

        register_at(&path, "/repo/a").expect("register 1");
        register_at(&path, "/repo/b").expect("register 2");
        register_at(&path, "/repo/a").expect("register duplicado");

        let paths = read_all_at(&path).expect("list");
        assert_eq!(paths, vec!["/repo/a".to_string(), "/repo/b".to_string()]);
    }
}
