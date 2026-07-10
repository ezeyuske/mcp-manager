mod adapters;
mod changelog;
mod commands;
mod disabled;
mod domain;
mod error;
mod mutations;
mod paths;
mod projects;
mod safe_write;
mod vault;

use commands::{
    bind_env_secret, copy_mcp, delete_mcp, duplicate_mcp, get_inventory, list_changelog,
    register_project_dir, restore_backup, set_mcp_enabled, unbind_env_secret, upsert_mcp,
    vault_delete_secret, vault_list, vault_reveal, vault_set_secret,
};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_shell::init())
        .invoke_handler(tauri::generate_handler![
            get_inventory,
            upsert_mcp,
            delete_mcp,
            duplicate_mcp,
            set_mcp_enabled,
            copy_mcp,
            list_changelog,
            restore_backup,
            register_project_dir,
            vault_list,
            vault_set_secret,
            vault_delete_secret,
            vault_reveal,
            bind_env_secret,
            unbind_env_secret
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
