use std::path::{Path, PathBuf};

use tempfile::NamedTempFile;

use crate::error::WriteError;
use crate::paths::backups_dir;

/// Copia `path` a `~/.mcp-manager/backups/<slug>/<basename>.<timestamp>.json`
/// si `path` existe. Si no existe (p. ej. primer ADD sobre un archivo que
/// todavía no fue creado), no hay nada que respaldar y devolvemos `None`.
///
/// El timestamp es compacto y ordenable: `YYYYMMDD-HHMMSS` (UTC).
pub fn backup(path: &Path, slug: &str) -> Result<Option<PathBuf>, WriteError> {
    if !path.exists() {
        return Ok(None);
    }

    let dir = backups_dir()?.join(slug);
    std::fs::create_dir_all(&dir).map_err(|source| WriteError::Backup {
        path: dir.display().to_string(),
        source,
    })?;

    let basename = path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "config".to_string());

    let timestamp = chrono::Utc::now().format("%Y%m%d-%H%M%S").to_string();
    let backup_path = dir.join(format!("{basename}.{timestamp}.json"));

    std::fs::copy(path, &backup_path).map_err(|source| WriteError::Backup {
        path: backup_path.display().to_string(),
        source,
    })?;

    Ok(Some(backup_path))
}

/// Escritura atómica: escribe `contents` en un archivo temporal creado en
/// el MISMO directorio que `path` (para garantizar que el `rename` final
/// sea atómico incluso a través de distintos filesystems montados) y lo
/// renombra sobre `path`.
///
/// Crea el directorio padre si falta.
pub fn atomic_write(path: &Path, contents: &str) -> Result<(), WriteError> {
    let parent = path.parent().ok_or_else(|| WriteError::Io {
        path: path.display().to_string(),
        source: std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "el path de destino no tiene directorio padre",
        ),
    })?;

    std::fs::create_dir_all(parent).map_err(|source| WriteError::Io {
        path: parent.display().to_string(),
        source,
    })?;

    let mut tmp = NamedTempFile::new_in(parent).map_err(|source| WriteError::Io {
        path: parent.display().to_string(),
        source,
    })?;

    use std::io::Write;
    tmp.write_all(contents.as_bytes())
        .map_err(|source| WriteError::Io {
            path: path.display().to_string(),
            source,
        })?;

    tmp.flush().map_err(|source| WriteError::Io {
        path: path.display().to_string(),
        source,
    })?;

    tmp.persist(path).map_err(|persist_err| WriteError::Io {
        path: path.display().to_string(),
        source: persist_err.error,
    })?;

    Ok(())
}

/// Pipeline seguro completo para escribir un `serde_json::Value` en un
/// archivo de config externo:
///
/// 1. Serializar pretty + newline final.
/// 2. VALIDAR reparseando el string producido (aborta antes de tocar
///    disco si algo salió mal).
/// 3. Backup timestampeado del contenido previo (si existe).
/// 4. Escritura atómica (tmp en el mismo dir + rename).
///
/// Devuelve el path del backup creado (si lo hubo).
pub fn write_json(
    path: &Path,
    value: &serde_json::Value,
    slug: &str,
) -> Result<Option<PathBuf>, WriteError> {
    let mut serialized =
        serde_json::to_string_pretty(value).map_err(|e| WriteError::Serialize {
            message: e.to_string(),
        })?;
    serialized.push('\n');

    // Validación: el string que vamos a escribir debe volver a parsear a
    // un JSON válido antes de que toquemos el disco.
    let reparsed: serde_json::Value =
        serde_json::from_str(&serialized).map_err(|e| WriteError::Validation {
            path: path.display().to_string(),
            message: e.to_string(),
        })?;
    if &reparsed != value {
        return Err(WriteError::Validation {
            path: path.display().to_string(),
            message: "el JSON reparseado no coincide con el valor original".to_string(),
        });
    }

    let backup_path = backup(path, slug)?;

    atomic_write(path, &serialized)?;

    Ok(backup_path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn atomic_write_produces_identical_content() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("config.json");

        atomic_write(&path, "hello\n").expect("write debe funcionar");

        let read_back = std::fs::read_to_string(&path).expect("debe poder leerse");
        assert_eq!(read_back, "hello\n");
    }

    #[test]
    fn atomic_write_creates_parent_dir_if_missing() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("nested").join("config.json");

        atomic_write(&path, "{}\n").expect("write debe crear el dir padre");

        assert!(path.exists());
    }

    #[test]
    fn backup_returns_none_when_target_does_not_exist() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("does-not-exist.json");

        let result = backup(&path, "test-slug").expect("no debe fallar");
        assert!(result.is_none());
    }

    #[test]
    fn write_json_validates_before_touching_disk() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("config.json");

        std::fs::write(&path, "{\"original\":true}\n").expect("setup");

        let value = json!({ "mcpServers": {} });
        let backup_path = write_json(&path, &value, "test-slug").expect("debe escribir");

        assert!(backup_path.is_some());
        let written: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&path).expect("leer")).expect("parsea");
        assert_eq!(written, value);

        // El backup preserva el contenido original.
        let backup_content = std::fs::read_to_string(backup_path.unwrap()).expect("leer backup");
        assert!(backup_content.contains("original"));
    }
}
