use std::ffi::OsString;
use std::path::PathBuf;

use crate::error::WriteError;

/// Env var de aislamiento para DESARROLLO. Si está seteada y no vacía,
/// redirige TODA la resolución de paths de la app (configs ajenos
/// incluidos) a un directorio sandbox. Objetivo: correr `pnpm tauri dev`
/// sin riesgo de tocar los configs reales de Claude Desktop/Code ni el
/// `~/.mcp-manager` real del usuario.
///
/// Layout del sandbox bajo `<root>`:
///   `<root>/home`   reemplaza a `dirs::home_dir()`
///                   (`~/.claude.json`, `~/.claude/skills`, `~/.mcp-manager`)
///   `<root>/config` reemplaza a `dirs::config_dir()`
///                   (config de Claude Desktop)
///
/// Al enrutar tanto la lectura (adapters) como la escritura (mutations)
/// por estas funciones, ambas capas quedan siempre consistentes.
pub const CONFIG_ROOT_ENV: &str = "MCP_MANAGER_CONFIG_ROOT";

/// Home efectivo del usuario. En modo normal, `dirs::home_dir()`; con el
/// sandbox activo, `<root>/home`.
pub fn home_dir() -> Option<PathBuf> {
    with_sandbox(read_sandbox_root(), dirs::home_dir(), "home")
}

/// Directorio de config del sistema. En modo normal, `dirs::config_dir()`
/// (Application Support en macOS, %APPDATA% en Windows); con el sandbox
/// activo, `<root>/config`.
pub fn config_dir() -> Option<PathBuf> {
    with_sandbox(read_sandbox_root(), dirs::config_dir(), "config")
}

/// `true` si el sandbox de desarrollo está activo.
pub fn sandbox_active() -> bool {
    read_sandbox_root().is_some()
}

fn read_sandbox_root() -> Option<PathBuf> {
    let parsed = sandbox_root_from(std::env::var_os(CONFIG_ROOT_ENV));
    reject_real_dirs(parsed, dirs::home_dir(), dirs::config_dir())
}

/// Núcleo puro: interpreta el valor crudo de la env var. Ausente, vacío o
/// solo-espacios => sin sandbox (`None`). Recorta whitespace de los bordes
/// cuando el valor es UTF-8 válido (un path con espacios accidentales al
/// principio/fin casi siempre es un error de shell).
fn sandbox_root_from(raw: Option<OsString>) -> Option<PathBuf> {
    let raw = raw?;
    match raw.to_str() {
        Some(s) => {
            let trimmed = s.trim();
            if trimmed.is_empty() {
                None
            } else {
                Some(PathBuf::from(trimmed))
            }
        }
        // Path no-UTF8: lo usamos tal cual (no podemos trim con seguridad).
        None => Some(PathBuf::from(raw)),
    }
}

/// Núcleo puro y GUARD de seguridad: un sandbox root que coincide con el
/// home o el config real del sistema NO aísla nada — sería tocar los datos
/// reales creyendo que están sandboxeados (el mismo tipo de accidente
/// silencioso que motivó esta feature). En ese caso lo descartamos
/// (`None` => sin sandbox), y como consecuencia el banner de arranque no
/// aparecerá: señal visible de que la env var apunta mal.
fn reject_real_dirs(
    root: Option<PathBuf>,
    real_home: Option<PathBuf>,
    real_config: Option<PathBuf>,
) -> Option<PathBuf> {
    let root = root?;
    if Some(&root) == real_home.as_ref() || Some(&root) == real_config.as_ref() {
        return None;
    }
    Some(root)
}

/// Núcleo puro: con sandbox root, devuelve `<root>/<sub>`; sin él, el path
/// real resuelto por `dirs`.
fn with_sandbox(root: Option<PathBuf>, real: Option<PathBuf>, sub: &str) -> Option<PathBuf> {
    match root {
        Some(r) => Some(r.join(sub)),
        None => real,
    }
}

/// Directorio propio de la app: `~/.mcp-manager/` (o `<root>/home/.mcp-manager`
/// con el sandbox activo). Se crea si no existe.
///
/// Nunca hardcodeamos `~`: resolvemos el home vía [`home_dir`].
pub fn app_data_dir() -> Result<PathBuf, WriteError> {
    let home = home_dir().ok_or_else(|| WriteError::NotSupported {
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

/// `~/.mcp-manager/builtin.json` (estado del MCP built-in propio de la
/// app: si está activo y en qué clientes — ver `builtin.rs`).
pub fn builtin_file() -> Result<PathBuf, WriteError> {
    Ok(app_data_dir()?.join("builtin.json"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sandbox_root_absent_or_empty_is_none() {
        assert_eq!(sandbox_root_from(None), None);
        assert_eq!(sandbox_root_from(Some(OsString::new())), None);
    }

    #[test]
    fn sandbox_root_present_parses_path() {
        assert_eq!(
            sandbox_root_from(Some(OsString::from("/tmp/box"))),
            Some(PathBuf::from("/tmp/box"))
        );
    }

    #[test]
    fn sandbox_root_whitespace_only_is_none() {
        assert_eq!(sandbox_root_from(Some(OsString::from("   "))), None);
    }

    #[test]
    fn sandbox_root_trims_surrounding_whitespace() {
        assert_eq!(
            sandbox_root_from(Some(OsString::from("  /tmp/box\n"))),
            Some(PathBuf::from("/tmp/box"))
        );
    }

    #[test]
    fn reject_real_dirs_drops_root_equal_to_home() {
        let home = Some(PathBuf::from("/Users/x"));
        let config = Some(PathBuf::from("/Users/x/Library/Application Support"));
        // root == home real => descartado (sin sandbox).
        assert_eq!(
            reject_real_dirs(
                Some(PathBuf::from("/Users/x")),
                home.clone(),
                config.clone()
            ),
            None
        );
        // root == config real => descartado.
        assert_eq!(
            reject_real_dirs(
                Some(PathBuf::from("/Users/x/Library/Application Support")),
                home,
                config
            ),
            None
        );
    }

    #[test]
    fn reject_real_dirs_keeps_distinct_root() {
        let root = Some(PathBuf::from("/tmp/box"));
        assert_eq!(
            reject_real_dirs(
                root.clone(),
                Some(PathBuf::from("/Users/x")),
                Some(PathBuf::from("/Users/x/Library"))
            ),
            root
        );
    }

    #[test]
    fn without_sandbox_uses_real_path() {
        let real = Some(PathBuf::from("/Users/x"));
        assert_eq!(with_sandbox(None, real.clone(), "home"), real);
    }

    #[test]
    fn with_sandbox_reroutes_under_subdir() {
        let root = Some(PathBuf::from("/tmp/box"));
        assert_eq!(
            with_sandbox(root.clone(), Some(PathBuf::from("/Users/x")), "home"),
            Some(PathBuf::from("/tmp/box/home"))
        );
        assert_eq!(
            with_sandbox(root, Some(PathBuf::from("/Users/x/Library")), "config"),
            Some(PathBuf::from("/tmp/box/config"))
        );
    }

    #[test]
    fn sandbox_wins_even_if_real_is_none() {
        // Aunque `dirs` no pueda resolver el path real (None), el sandbox
        // sigue mandando: la resolución no depende del entorno del host.
        assert_eq!(
            with_sandbox(Some(PathBuf::from("/tmp/box")), None, "home"),
            Some(PathBuf::from("/tmp/box/home"))
        );
    }
}
