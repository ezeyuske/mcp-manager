use serde::{Deserialize, Serialize};

use crate::domain::{AppId, Scope};
use crate::error::WriteError;
use crate::paths::changelog_file;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MutationAction {
    Add,
    Edit,
    Delete,
    Duplicate,
    Rename,
    Enable,
    Disable,
    Copy,
    Restore,
    VaultSet,
    VaultDelete,
    Bind,
    Unbind,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MutationLog {
    pub id: String,
    pub timestamp: String,
    pub app: AppId,
    pub scope: Scope,
    pub file_path: String,
    pub action: MutationAction,
    pub mcp_name: String,
    pub backup_path: Option<String>,
}

impl MutationLog {
    pub fn new(
        app: AppId,
        scope: Scope,
        file_path: String,
        action: MutationAction,
        mcp_name: String,
        backup_path: Option<String>,
    ) -> Self {
        let timestamp = chrono::Utc::now().to_rfc3339();
        let ts_millis = chrono::Utc::now().timestamp_millis();
        let id = format!("{ts_millis}-{mcp_name}");

        MutationLog {
            id,
            timestamp,
            app,
            scope,
            file_path,
            action,
            mcp_name,
            backup_path,
        }
    }
}

fn read_all_at(path: &std::path::Path) -> Result<Vec<MutationLog>, WriteError> {
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

/// Agrega una entrada al final del changelog (append), persistiendo todo
/// el array. No usa `safe_write::write_json` porque este archivo es
/// propio de la app (no un config externo de terceros).
fn append_at(path: &std::path::Path, entry: MutationLog) -> Result<(), WriteError> {
    let mut entries = read_all_at(path)?;
    entries.push(entry);

    let serialized = serde_json::to_string_pretty(&entries).map_err(|e| WriteError::Serialize {
        message: e.to_string(),
    })?;

    std::fs::write(path, serialized).map_err(|source| WriteError::Io {
        path: path.display().to_string(),
        source,
    })
}

/// Agrega una entrada al changelog de la app (`~/.mcp-manager/changelog.json`).
pub fn append(entry: MutationLog) -> Result<(), WriteError> {
    append_at(&changelog_file()?, entry)
}

/// Devuelve todas las entradas del changelog de la app, más nueva primero.
pub fn list() -> Result<Vec<MutationLog>, WriteError> {
    let mut entries = read_all_at(&changelog_file()?)?;
    entries.reverse();
    Ok(entries)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mutation_log_serializes_action_as_snake_case() {
        let entry = MutationLog::new(
            AppId::ClaudeDesktop,
            Scope::User,
            "/tmp/x.json".to_string(),
            MutationAction::Add,
            "context7".to_string(),
            Some("/tmp/backup.json".to_string()),
        );

        let value = serde_json::to_value(&entry).expect("serializa");
        assert_eq!(value["action"], "add");
        assert_eq!(value["mcpName"], "context7");
        assert_eq!(value["backupPath"], "/tmp/backup.json");
        assert_eq!(value["filePath"], "/tmp/x.json");
    }

    #[test]
    fn append_then_list_round_trips_and_orders_newest_first() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("changelog.json");

        let first = MutationLog::new(
            AppId::ClaudeDesktop,
            Scope::User,
            "/tmp/a.json".to_string(),
            MutationAction::Add,
            "context7".to_string(),
            Some("/tmp/backup-1.json".to_string()),
        );
        let second = MutationLog::new(
            AppId::ClaudeCode,
            Scope::Project,
            "/tmp/b/.mcp.json".to_string(),
            MutationAction::Delete,
            "hibob".to_string(),
            None,
        );

        append_at(&path, first.clone()).expect("append 1");
        append_at(&path, second.clone()).expect("append 2");

        let mut entries = read_all_at(&path).expect("read_all");
        entries.reverse();

        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].mcp_name, "hibob");
        assert_eq!(entries[1].mcp_name, "context7");
        assert_eq!(
            entries[1].backup_path.as_deref(),
            Some("/tmp/backup-1.json")
        );
    }
}
