//! Vault de secrets: los VALORES viven exclusivamente en el keychain del
//! OS (crate `keyring`), nunca en disco ni en el estado del frontend.
//!
//! `~/.mcp-manager/vault.json` guarda SOLO metadata (nombres de secretos
//! y bindings hacia MCPs concretos) — jamás un valor. Ver `write_all_at`
//! para el porqué de la escritura directa (no `safe_write`).
//!
//! Principio de mínimo privilegio: un secreto se inyecta SOLO en los
//! MCPs cuyo binding lo referencia explícitamente. Cualquier función que
//! "propague" un secreto (`set_secret`) filtra `bindings` por
//! `secret_name` antes de tocar un solo config.

use std::path::Path;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::changelog::{self, MutationAction, MutationLog};
use crate::error::WriteError;
use crate::mutations::{self, McpTarget};
use crate::paths::vault_file;

const SERVICE: &str = "mcp-manager";

// ---------------------------------------------------------------------
// Modelo de `vault.json`. SOLO metadata: nombres de secretos existentes
// y bindings (qué envKey de qué target usa qué secretName). El valor del
// secreto NUNCA aparece acá.
// ---------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VaultBinding {
    pub target: McpTarget,
    pub env_key: String,
    pub secret_name: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct VaultFile {
    #[serde(default)]
    secrets: Vec<String>,
    #[serde(default)]
    bindings: Vec<VaultBinding>,
}

/// DTO expuesto al frontend por `vault_list`.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VaultSecretInfo {
    pub name: String,
    pub used_by: usize,
}

fn read_all_at(path: &Path) -> Result<VaultFile, WriteError> {
    if !path.exists() {
        return Ok(VaultFile::default());
    }

    let raw = std::fs::read_to_string(path).map_err(|source| WriteError::Io {
        path: path.display().to_string(),
        source,
    })?;

    if raw.trim().is_empty() {
        return Ok(VaultFile::default());
    }

    serde_json::from_str(&raw).map_err(|e| WriteError::Validation {
        path: path.display().to_string(),
        message: e.to_string(),
    })
}

/// Escritura directa (sin `safe_write`): igual que `disabled.json`,
/// `changelog.json` y `projects.json`, `vault.json` es estado INTERNO de
/// mcp-manager (además, nunca contiene valores sensibles — solo nombres
/// y bindings), no un config ajeno de terceros que amerite el
/// backup+atomic-write reservado para esos casos. El peor escenario de
/// corrupción es perder qué MCP tenía qué binding, nunca dañar un config
/// real ni exponer un secreto.
fn write_all_at(path: &Path, file: &VaultFile) -> Result<(), WriteError> {
    let serialized = serde_json::to_string_pretty(file).map_err(|e| WriteError::Serialize {
        message: e.to_string(),
    })?;

    std::fs::write(path, serialized).map_err(|source| WriteError::Io {
        path: path.display().to_string(),
        source,
    })
}

// ---------------------------------------------------------------------
// Capa mínima sobre `keyring`, con degradación a error claro (nunca
// panic) si el backend nativo no está disponible (p. ej. Linux sin
// Secret Service corriendo).
// ---------------------------------------------------------------------

fn entry(name: &str) -> Result<keyring::Entry, WriteError> {
    keyring::Entry::new(SERVICE, name).map_err(|source| WriteError::Keychain {
        message: format!("no se pudo abrir el keychain para '{name}': {source}"),
    })
}

fn keychain_set(name: &str, value: &str) -> Result<(), WriteError> {
    entry(name)?
        .set_password(value)
        .map_err(|source| WriteError::Keychain {
            message: format!("no se pudo guardar el secreto '{name}' en el keychain: {source}"),
        })
}

fn keychain_get(name: &str) -> Result<String, WriteError> {
    entry(name)?
        .get_password()
        .map_err(|source| WriteError::Keychain {
            message: format!("no se pudo leer el secreto '{name}' del keychain: {source}"),
        })
}

fn keychain_delete(name: &str) -> Result<(), WriteError> {
    match entry(name)?.delete_credential() {
        Ok(()) => Ok(()),
        // Borrar algo que ya no existe no debe ser un error duro: el
        // estado final deseado (el secreto no vive en el keychain) ya se
        // cumple.
        Err(keyring::Error::NoEntry) => Ok(()),
        Err(source) => Err(WriteError::Keychain {
            message: format!("no se pudo borrar el secreto '{name}' del keychain: {source}"),
        }),
    }
}

// ---------------------------------------------------------------------
// Inyección/limpieza de env en el config de UN target puntual.
// ---------------------------------------------------------------------

/// Reescribe SOLO el config de `target`, seteando `env[env_key] = value`.
/// Usa `mutations::read_entry_value` + `upsert_raw_value` (en vez de
/// `mutations::upsert`) para preservar exactamente cualquier otro campo
/// de la entrada (incluido cualquier otra clave de `env`) — mismo
/// patrón que `disabled.rs` usa para no perder campos desconocidos.
fn inject_env_value(target: &McpTarget, env_key: &str, value: &str) -> Result<(), WriteError> {
    let mut entry_value = mutations::read_entry_value(target).unwrap_or(Value::Null);
    if !entry_value.is_object() {
        entry_value = Value::Object(serde_json::Map::new());
    }

    let obj = entry_value
        .as_object_mut()
        .expect("acabamos de garantizar que es un objeto");

    let env_obj = obj
        .entry("env")
        .or_insert_with(|| Value::Object(serde_json::Map::new()));
    if !env_obj.is_object() {
        *env_obj = Value::Object(serde_json::Map::new());
    }
    env_obj
        .as_object_mut()
        .expect("acabamos de garantizar que env es un objeto")
        .insert(env_key.to_string(), Value::String(value.to_string()));

    mutations::upsert_raw_value(target, entry_value)?;
    Ok(())
}

/// Reescribe SOLO el config de `target`, quitando la clave `env_key` de
/// su `env` (si existe). El resto de la entrada (incluido el resto de
/// `env`) se preserva intacto.
fn remove_env_value(target: &McpTarget, env_key: &str) -> Result<(), WriteError> {
    let Ok(mut entry_value) = mutations::read_entry_value(target) else {
        // El target ya no existe (p. ej. se borró el MCP): nada que
        // limpiar en el config real.
        return Ok(());
    };

    if let Some(obj) = entry_value.as_object_mut() {
        if let Some(env_obj) = obj.get_mut("env").and_then(Value::as_object_mut) {
            env_obj.remove(env_key);
            if env_obj.is_empty() {
                obj.remove("env");
            }
        }
    }

    mutations::upsert_raw_value(target, entry_value)?;
    Ok(())
}

fn log(action: MutationAction, target: Option<&McpTarget>, extra_name: &str) {
    let (app, scope, name, file_path) = match target {
        Some(t) => {
            let file_path = mutations::resolve_target_path(t)
                .map(|p| p.display().to_string())
                .unwrap_or_default();
            (t.app, t.scope, t.name.clone(), file_path)
        }
        None => (
            crate::domain::AppId::ClaudeCode,
            crate::domain::Scope::User,
            extra_name.to_string(),
            String::new(),
        ),
    };

    let entry = MutationLog::new(app, scope, file_path, action, name, None);
    // Best effort, igual que el resto de las mutaciones: el changelog
    // nunca debe enmascarar u ocultar que la operación real (keychain o
    // config) ya sucedió con éxito.
    let _ = changelog::append(entry);
}

// ---------------------------------------------------------------------
// API pública del vault.
// ---------------------------------------------------------------------

/// Da de alta (o rota) un secreto: lo guarda en el keychain y lo agrega a
/// `vault.json.secrets`. Luego PROPAGA el valor solo a los bindings ya
/// existentes que referencian este `secret_name` (mínimo privilegio: los
/// MCPs no vinculados nunca lo reciben).
pub fn set_secret(name: &str, value: &str) -> Result<(), WriteError> {
    set_secret_at(&vault_file()?, name, value)
}

fn set_secret_at(vault_path: &Path, name: &str, value: &str) -> Result<(), WriteError> {
    keychain_set(name, value)?;

    let mut file = read_all_at(vault_path)?;
    if !file.secrets.iter().any(|s| s == name) {
        file.secrets.push(name.to_string());
    }
    write_all_at(vault_path, &file)?;

    for binding in file.bindings.iter().filter(|b| b.secret_name == name) {
        inject_env_value(&binding.target, &binding.env_key, value)?;
    }

    log(MutationAction::VaultSet, None, name);
    Ok(())
}

/// Lee el valor de un secreto directamente del keychain.
pub fn get_secret(name: &str) -> Result<String, WriteError> {
    keychain_get(name)
}

/// Igual que `get_secret`: expuesto con nombre propio porque, del lado
/// del frontend, "revelar" (mostrar bajo demanda) es una acción distinta
/// conceptualmente de "usar internamente para inyectar en un config",
/// aunque la implementación sea la misma lectura del keychain.
pub fn reveal(name: &str) -> Result<String, WriteError> {
    get_secret(name)
}

/// Borra un secreto: limpia `env[envKey]` de cada config vinculado,
/// quita sus bindings y el nombre de `vault.json`, y lo borra del
/// keychain.
pub fn delete_secret(name: &str) -> Result<(), WriteError> {
    delete_secret_at(&vault_file()?, name)
}

fn delete_secret_at(vault_path: &Path, name: &str) -> Result<(), WriteError> {
    let mut file = read_all_at(vault_path)?;

    let (to_clean, remaining): (Vec<_>, Vec<_>) = file
        .bindings
        .into_iter()
        .partition(|b| b.secret_name == name);

    for binding in &to_clean {
        remove_env_value(&binding.target, &binding.env_key)?;
    }

    file.bindings = remaining;
    file.secrets.retain(|s| s != name);
    write_all_at(vault_path, &file)?;

    keychain_delete(name)?;

    log(MutationAction::VaultDelete, None, name);
    Ok(())
}

/// Lista los secretos conocidos con cuántos bindings los usan.
pub fn list_secrets() -> Result<Vec<VaultSecretInfo>, WriteError> {
    list_secrets_at(&vault_file()?)
}

fn list_secrets_at(vault_path: &Path) -> Result<Vec<VaultSecretInfo>, WriteError> {
    let file = read_all_at(vault_path)?;

    Ok(file
        .secrets
        .iter()
        .map(|name| VaultSecretInfo {
            name: name.clone(),
            used_by: file.bindings.iter().filter(|b| &b.secret_name == name).count(),
        })
        .collect())
}

/// Vincula `env_key` del config de `target` a `secret_name`: agrega el
/// binding en `vault.json` y reescribe SOLO ese config, inyectando el
/// valor actual del secreto.
pub fn bind(target: &McpTarget, env_key: &str, secret_name: &str) -> Result<(), WriteError> {
    bind_at(&vault_file()?, target, env_key, secret_name)
}

fn bind_at(
    vault_path: &Path,
    target: &McpTarget,
    env_key: &str,
    secret_name: &str,
) -> Result<(), WriteError> {
    let value = get_secret(secret_name)?;

    let mut file = read_all_at(vault_path)?;
    file.bindings
        .retain(|b| !(b.target == *target && b.env_key == env_key));
    file.bindings.push(VaultBinding {
        target: target.clone(),
        env_key: env_key.to_string(),
        secret_name: secret_name.to_string(),
    });
    write_all_at(vault_path, &file)?;

    inject_env_value(target, env_key, &value)?;

    log(MutationAction::Bind, Some(target), secret_name);
    Ok(())
}

/// Desvincula `env_key` del config de `target`: quita el binding y
/// reescribe SOLO ese config, quitando esa clave de su `env`.
pub fn unbind(target: &McpTarget, env_key: &str) -> Result<(), WriteError> {
    unbind_at(&vault_file()?, target, env_key)
}

fn unbind_at(vault_path: &Path, target: &McpTarget, env_key: &str) -> Result<(), WriteError> {
    let mut file = read_all_at(vault_path)?;
    file.bindings
        .retain(|b| !(b.target == *target && b.env_key == env_key));
    write_all_at(vault_path, &file)?;

    remove_env_value(target, env_key)?;

    log(MutationAction::Unbind, Some(target), env_key);
    Ok(())
}

/// Bindings activos para un target puntual: `(envKey, secretName)`. Usado
/// por `mutations::upsert` (para inyectar vault-aware) y por
/// `get_inventory` (para poblar `McpInstallation.vault_keys`).
pub fn bindings_for_target(target: &McpTarget) -> Result<Vec<(String, String)>, WriteError> {
    bindings_for_target_at(&vault_file()?, target)
}

fn bindings_for_target_at(
    vault_path: &Path,
    target: &McpTarget,
) -> Result<Vec<(String, String)>, WriteError> {
    let file = read_all_at(vault_path)?;
    Ok(file
        .bindings
        .into_iter()
        .filter(|b| b.target == *target)
        .map(|b| (b.env_key, b.secret_name))
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{AppId, McpServerConfig, Scope};
    use serde_json::json;
    use std::sync::atomic::{AtomicU64, Ordering};

    /// NOTA IMPORTANTE sobre el aislamiento en estos tests: a diferencia
    /// del resto de Fase 3 (que aísla `$HOME` con `set_var` para no
    /// tocar el filesystem real del usuario), los tests de este módulo
    /// ejercitan el keychain REAL del OS a propósito (así lo pide el
    /// plan). Aislar `$HOME` rompe eso: en macOS, Security Framework
    /// resuelve el "default keychain" desde el home real de la sesión
    /// de login, y bajo un `$HOME` de tempdir directamente no encuentra
    /// ninguno ("A default keychain could not be found"), haciendo
    /// fallar CUALQUIER operación de keychain, real o de test.
    ///
    /// Por eso acá NO tocamos `$HOME` NI el `vault.json` real
    /// (`~/.mcp-manager/vault.json`, que es compartido entre TODOS los
    /// tests del proceso y con `cargo test` corriendo en paralelo por
    /// threads se pisaba entre corridas — la causa del flake original).
    /// Cada test usa:
    /// - Su propio `vault.json` en un tempdir propio, vía las variantes
    ///   `_at` (`set_secret_at`/`delete_secret_at`/`bind_at`/etc.), nunca
    ///   la API pública (`set_secret`/`bind`/...) que resuelve al path
    ///   real compartido.
    /// - `Scope::Project` para los configs (resuelve a
    ///   `<project_path>/.mcp.json` en su propio tempdir, sin pasar por
    ///   `dirs::home_dir()`/`config_dir()`).
    /// - Un nombre de secreto namespaced y ÚNICO (contador atómico +
    ///   timestamp) para el keychain real, que sí es global del OS y
    ///   puede correr en paralelo entre tests sin colisionar — y se
    ///   borra siempre al final.
    fn project_target(project_dir: &Path, name: &str) -> McpTarget {
        McpTarget {
            app: AppId::ClaudeCode,
            scope: Scope::Project,
            project_path: Some(project_dir.display().to_string()),
            name: name.to_string(),
        }
    }

    static SECRET_COUNTER: AtomicU64 = AtomicU64::new(0);

    fn spike_secret_name(case: &str) -> String {
        let counter = SECRET_COUNTER.fetch_add(1, Ordering::SeqCst);
        format!(
            "mcp-manager-test-{case}-{}-{counter}",
            chrono::Utc::now().timestamp_nanos_opt().unwrap_or_default()
        )
    }

    /// `vault.json` propio de este test, en su propio tempdir. Devuelve
    /// el tempdir (para mantenerlo vivo durante el test) y el path.
    fn own_vault() -> (tempfile::TempDir, std::path::PathBuf) {
        let dir = tempfile::tempdir().expect("tempdir vault");
        let path = dir.path().join("vault.json");
        (dir, path)
    }

    #[test]
    fn vault_json_never_contains_secret_values() {
        let (_vault_dir, vault_path) = own_vault();
        let project_dir = tempfile::tempdir().expect("tempdir proyecto");
        std::fs::write(
            project_dir.path().join(".mcp.json"),
            r#"{"mcpServers": {"srv": {"command": "npx"}}}"#,
        )
        .expect("setup .mcp.json");

        let secret_name = spike_secret_name("no-values");
        let target = project_target(project_dir.path(), "srv");

        set_secret_at(&vault_path, &secret_name, "super-secret-value-xyz").expect("set_secret");
        bind_at(&vault_path, &target, "API_KEY", &secret_name).expect("bind");

        let raw = std::fs::read_to_string(&vault_path).expect("leer vault.json");
        assert!(!raw.contains("super-secret-value-xyz"));

        delete_secret_at(&vault_path, &secret_name).expect("cleanup delete_secret");
    }

    #[test]
    fn bind_injects_value_only_into_target_config_not_others() {
        let (_vault_dir, vault_path) = own_vault();
        let project_dir = tempfile::tempdir().expect("tempdir proyecto");
        std::fs::write(
            project_dir.path().join(".mcp.json"),
            r#"{"mcpServers": {"bound": {"command": "npx"}, "other": {"command": "npx"}}}"#,
        )
        .expect("setup .mcp.json");

        let secret_name = spike_secret_name("bind-scoped");
        set_secret_at(&vault_path, &secret_name, "value-for-bound-only").expect("set_secret");

        let bound_target = project_target(project_dir.path(), "bound");
        bind_at(&vault_path, &bound_target, "TOKEN", &secret_name).expect("bind");

        let config_path = mutations::resolve_target_path(&bound_target).unwrap();
        let written: Value =
            serde_json::from_str(&std::fs::read_to_string(&config_path).unwrap()).unwrap();

        assert_eq!(written["mcpServers"]["bound"]["env"]["TOKEN"], "value-for-bound-only");
        assert!(written["mcpServers"]["other"].get("env").is_none());

        delete_secret_at(&vault_path, &secret_name).expect("cleanup delete_secret");
    }

    #[test]
    fn set_secret_propagates_to_bound_mcps_but_not_to_unbound_third_mcp() {
        let (_vault_dir, vault_path) = own_vault();
        let project_dir = tempfile::tempdir().expect("tempdir proyecto");
        std::fs::write(
            project_dir.path().join(".mcp.json"),
            r#"{
                "mcpServers": {
                    "mcp-a": {"command": "npx"},
                    "mcp-b": {"command": "npx"},
                    "mcp-c": {"command": "npx"}
                }
            }"#,
        )
        .expect("setup .mcp.json");

        let secret_name = spike_secret_name("least-priv");

        // Bindeamos a/b (con un valor placeholder previo, luego rotamos
        // vía set_secret) pero NO c.
        set_secret_at(&vault_path, &secret_name, "initial-value").expect("set_secret inicial");
        bind_at(
            &vault_path,
            &project_target(project_dir.path(), "mcp-a"),
            "SHARED_TOKEN",
            &secret_name,
        )
        .expect("bind a");
        bind_at(
            &vault_path,
            &project_target(project_dir.path(), "mcp-b"),
            "SHARED_TOKEN",
            &secret_name,
        )
        .expect("bind b");

        // Rotamos el secreto: debe propagar solo a a y b.
        set_secret_at(&vault_path, &secret_name, "rotated-value").expect("set_secret rotado");

        let config_path =
            mutations::resolve_target_path(&project_target(project_dir.path(), "mcp-a")).unwrap();
        let written: Value =
            serde_json::from_str(&std::fs::read_to_string(&config_path).unwrap()).unwrap();

        assert_eq!(written["mcpServers"]["mcp-a"]["env"]["SHARED_TOKEN"], "rotated-value");
        assert_eq!(written["mcpServers"]["mcp-b"]["env"]["SHARED_TOKEN"], "rotated-value");
        // mínimo privilegio: mcp-c nunca recibió el secreto.
        assert!(written["mcpServers"]["mcp-c"].get("env").is_none());

        delete_secret_at(&vault_path, &secret_name).expect("cleanup delete_secret");
    }

    #[test]
    fn unbind_clears_env_key_from_that_config() {
        let (_vault_dir, vault_path) = own_vault();
        let project_dir = tempfile::tempdir().expect("tempdir proyecto");
        std::fs::write(
            project_dir.path().join(".mcp.json"),
            r#"{"mcpServers": {"srv": {"command": "npx", "env": {"KEEP_ME": "1"}}}}"#,
        )
        .expect("setup .mcp.json");

        let secret_name = spike_secret_name("unbind");
        set_secret_at(&vault_path, &secret_name, "some-value").expect("set_secret");

        let target = project_target(project_dir.path(), "srv");
        bind_at(&vault_path, &target, "TO_REMOVE", &secret_name).expect("bind");

        unbind_at(&vault_path, &target, "TO_REMOVE").expect("unbind");

        let config_path = mutations::resolve_target_path(&target).unwrap();
        let written: Value =
            serde_json::from_str(&std::fs::read_to_string(&config_path).unwrap()).unwrap();

        assert!(written["mcpServers"]["srv"]["env"].get("TO_REMOVE").is_none());
        // Otras claves de env que no vienen del vault se preservan.
        assert_eq!(written["mcpServers"]["srv"]["env"]["KEEP_ME"], "1");

        assert!(bindings_for_target_at(&vault_path, &target)
            .unwrap()
            .is_empty());

        delete_secret_at(&vault_path, &secret_name).expect("cleanup delete_secret");
    }

    /// Único test que necesita el path REAL de `vault.json`:
    /// `mutations::upsert` (código de producción) llama internamente a la
    /// API pública `vault::bindings_for_target`/`vault::get_secret`, que
    /// resuelven al vault real vía `vault_file()` — no se puede
    /// parametrizar sin cambiar la firma pública de `mutations::upsert`
    /// (que no recibe ningún parámetro de vault, a propósito, para que
    /// cualquier caller de producción sea vault-aware sin tener que saber
    /// nada del vault). Por eso este test SÍ usa la API pública
    /// (`bind`/`set_secret`/`delete_secret`, que tocan
    /// `~/.mcp-manager/vault.json`) y se marca `#[serial]`
    /// (`serial_test`) para que nunca corra en paralelo con otro test
    /// que también toque ese archivo compartido — hoy ningún otro test
    /// lo hace, pero `serial` lo deja documentado y a prueba de
    /// regresiones futuras si se agrega otro test así.
    #[test]
    #[serial_test::serial(vault_real_file)]
    fn upsert_on_mcp_with_binding_injects_vault_value() {
        let project_dir = tempfile::tempdir().expect("tempdir proyecto");
        std::fs::write(project_dir.path().join(".mcp.json"), r#"{"mcpServers": {}}"#)
            .expect("setup .mcp.json");

        let secret_name = spike_secret_name("upsert-aware");
        let mcp_name = format!("srv-{secret_name}");
        set_secret(&secret_name, "upsert-value").expect("set_secret");

        let target = project_target(project_dir.path(), &mcp_name);
        // Bind antes de que el MCP exista todavía (bind ya crea/actualiza
        // la entrada vía upsert_raw_value).
        bind(&target, "API_KEY", &secret_name).expect("bind");

        // Un edit posterior del usuario (sin tocar env) no debe perder
        // el binding: mutations::upsert es vault-aware.
        let cfg: McpServerConfig = serde_json::from_value(json!({
            "command": "uv",
            "args": ["run", "srv"]
        }))
        .unwrap();
        mutations::upsert(&target, cfg).expect("upsert");

        let config_path = mutations::resolve_target_path(&target).unwrap();
        let written: Value =
            serde_json::from_str(&std::fs::read_to_string(&config_path).unwrap()).unwrap();

        assert_eq!(written["mcpServers"][&mcp_name]["env"]["API_KEY"], "upsert-value");
        assert_eq!(written["mcpServers"][&mcp_name]["args"][0], "run");

        delete_secret(&secret_name).expect("cleanup delete_secret");
    }

    /// No podemos simular fácilmente una falla real del backend nativo de
    /// keyring desde un test unitario portable, pero sí confirmamos que
    /// la ruta de error (nombre de entry inválido para el backend, o
    /// backend inaccesible) nunca panickea y siempre se propaga como
    /// `WriteError::Keychain` legible.
    #[test]
    fn keychain_get_of_nonexistent_secret_is_a_clear_error_not_a_panic() {
        let name = spike_secret_name("missing");
        let result = get_secret(&name);
        assert!(result.is_err());
        if let Err(err) = result {
            match err {
                WriteError::Keychain { message } => assert!(!message.is_empty()),
                other => panic!("se esperaba WriteError::Keychain, se obtuvo {other:?}"),
            }
        }
    }
}
