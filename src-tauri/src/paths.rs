use std::path::PathBuf;

use crate::error::WriteError;

/// Directorio propio de la app: `~/.mcp-manager/`. Se crea si no existe.
///
/// Nunca hardcodeamos `~`: resolvemos el home vía `dirs::home_dir()`.
pub fn app_data_dir() -> Result<PathBuf, WriteError> {
    let home = dirs::home_dir().ok_or_else(|| WriteError::NotSupported {
        message: "no se pudo resolver el directorio home del usuario".to_string(),
    })?;

    let dir = home.join(".mcp-manager");
    std::fs::create_dir_all(&dir).map_err(|source| WriteError::Io {
        path: dir.display().to_string(),
        source,
    })?;

    Ok(dir)
}

/// `~/.mcp-manager/backups/`. Se crea si no existe.
pub fn backups_dir() -> Result<PathBuf, WriteError> {
    let dir = app_data_dir()?.join("backups");
    std::fs::create_dir_all(&dir).map_err(|source| WriteError::Io {
        path: dir.display().to_string(),
        source,
    })?;
    Ok(dir)
}

/// `~/.mcp-manager/disabled.json` (sidecar de MCPs deshabilitados).
pub fn disabled_file() -> Result<PathBuf, WriteError> {
    Ok(app_data_dir()?.join("disabled.json"))
}

/// `~/.mcp-manager/changelog.json` (historial de mutaciones).
pub fn changelog_file() -> Result<PathBuf, WriteError> {
    Ok(app_data_dir()?.join("changelog.json"))
}

/// `~/.mcp-manager/projects.json` (rutas de repos conocidos).
pub fn projects_file() -> Result<PathBuf, WriteError> {
    Ok(app_data_dir()?.join("projects.json"))
}

/// `~/.mcp-manager/vault.json` (metadata de secrets: SOLO nombres y
/// bindings, nunca valores — ver `vault.rs`).
pub fn vault_file() -> Result<PathBuf, WriteError> {
    Ok(app_data_dir()?.join("vault.json"))
}
