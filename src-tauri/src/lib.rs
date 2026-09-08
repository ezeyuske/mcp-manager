// `pub` para que los tests de integración (`tests/`) puedan ejercitar la
// orquestación multi-target de los commands sin levantar Tauri.
pub mod commands;

use commands::{
    bind_env_secret, copy_mcp, delete_mcp, delete_skill, duplicate_mcp, get_builtin_status,
    get_inventory, get_skills, list_changelog, list_projects, read_env_value, register_project_dir,
    rename_mcp, rename_skill, restore_backup, set_builtin_enabled, set_mcp_enabled, set_mcp_env,
    set_skill_enabled, unbind_env_secret, unregister_project, upsert_mcp, upsert_skill,
    vault_delete_secret, vault_list, vault_reveal, vault_set_secret,
};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Aviso de arranque cuando el sandbox de desarrollo está activo: TODA
    // la resolución de paths (configs ajenos incluidos) va a un directorio
    // aislado y no se tocan los configs reales del usuario.
    if mcp_core::paths::sandbox_active() {
        eprintln!(
            "[mcp-manager] SANDBOX DE DEV ACTIVO ({}): los configs reales de \
             Claude Desktop/Code no serán tocados.",
            mcp_core::paths::CONFIG_ROOT_ENV
        );
    }

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_shell::init())
        .setup(|_app| {
            // Auto-heal del built-in: re-registra solo si el path del
            // sidecar cambió desde la última activación.
            commands::heal_builtin_on_startup();
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_inventory,
            upsert_mcp,
            delete_mcp,
            duplicate_mcp,
            rename_mcp,
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
            unbind_env_secret,
            read_env_value,
            set_mcp_env,
            unregister_project,
            list_projects,
            get_skills,
            set_skill_enabled,
            delete_skill,
            rename_skill,
            get_builtin_status,
            set_builtin_enabled,
            upsert_skill
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
