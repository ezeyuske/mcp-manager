//! Rename de un MCP: la única operación del proyecto que cambia la
//! IDENTIDAD de una entrada, no su contenido.
//!
//! El nombre de un MCP es la clave del mapa `mcpServers` y, a la vez,
//! parte de la tupla `McpTarget` con la que se indexan dos sidecars
//! internos. Cambiarlo en un solo lugar deja estado huérfano en silencio,
//! así que este módulo existe para que ese conjunto de pasos viva en un
//! único sitio, compartido por el backend Tauri y por el servidor MCP
//! standalone.
//!
//! Los pasos, por target:
//!
//! 1. re-keyear la entrada donde efectivamente viva: en el config ajeno
//!    (`mutations::rename`) o en el sidecar de deshabilitados
//!    (`disabled::rename`), porque un MCP deshabilitado NO existe en
//!    ningún config;
//! 2. reapuntar sus bindings del vault al target nuevo
//!    (`vault::retarget_bindings`).
//!
//! ORDEN DELIBERADO (config ajeno primero, estado interno después): si el
//! proceso muere en el medio, el peor caso es un binding desactualizado
//! —recuperable a mano, y visible porque el MCP pierde el candado en la
//! UI— y nunca un config de otra app corrupto. Mismo criterio que
//! `disabled::disable`.

use crate::disabled;
use crate::error::WriteError;
use crate::mutations::{self, McpTarget};
use crate::vault;

/// Dónde vive hoy la entrada que se va a renombrar.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Location {
    /// En el config real de la app.
    Config,
    /// En `~/.mcp-manager/disabled.json` (el MCP está deshabilitado).
    DisabledSidecar,
}

/// Valida SIN escribir nada que `target` se pueda renombrar a `new_name`,
/// y devuelve dónde vive la entrada para que `apply` no tenga que volver
/// a averiguarlo.
///
/// Se expone aparte de `apply` para que un rename sobre varios targets
/// pueda validarlos TODOS antes de tocar el primero: así el modo de falla
/// común (el nombre ya está tomado) aborta la operación completa sin
/// escribir, en vez de dejar la mitad de los configs renombrados.
pub fn preflight(target: &McpTarget, new_name: &str) -> Result<Location, WriteError> {
    mutations::valid_mcp_name(new_name)?;

    if disabled::contains(target)? {
        // La entrada vive en el sidecar, pero el nombre nuevo igual tiene
        // que estar libre en el config real: si no, el MCP quedaría
        // imposible de habilitar más tarde.
        if mutations::name_exists(target, new_name)? {
            return Err(WriteError::Conflict {
                path: mutations::resolve_target_path(target)?
                    .display()
                    .to_string(),
                message: format!("ya existe un MCP llamado '{new_name}'"),
            });
        }
        Ok(Location::DisabledSidecar)
    } else {
        mutations::rename_preflight(target, new_name)?;
        Ok(Location::Config)
    }
}

/// Aplica el rename de UN target. `location` viene de `preflight`.
///
/// El `Err` distingue los dos estados posibles de fallo mediante
/// `RenameError::vault_only`: si falla el paso 2, el nombre YA cambió y
/// quien llama tiene que decirlo así en vez de reportar que no pasó nada.
pub fn apply(
    target: &McpTarget,
    new_name: &str,
    location: Location,
) -> Result<McpTarget, RenameError> {
    let mut renamed = target.clone();
    renamed.name = new_name.to_string();

    match location {
        Location::Config => mutations::rename(target, new_name).map(|_| ()),
        Location::DisabledSidecar => disabled::rename(target, new_name),
    }
    .map_err(|source| RenameError {
        source,
        vault_only: false,
    })?;

    vault::retarget_bindings(target, &renamed).map_err(|source| RenameError {
        source,
        vault_only: true,
    })?;

    Ok(renamed)
}

/// Conveniencia para renombrar un único target de una sola llamada
/// (preflight + apply). El camino multi-target hace el preflight de todos
/// primero, así que usa `preflight` y `apply` por separado.
pub fn rename_one(target: &McpTarget, new_name: &str) -> Result<McpTarget, RenameError> {
    let location = preflight(target, new_name).map_err(|source| RenameError {
        source,
        vault_only: false,
    })?;
    apply(target, new_name, location)
}

/// Error de rename, con el dato clave de si el nombre llegó a cambiarse.
#[derive(Debug)]
pub struct RenameError {
    pub source: WriteError,
    /// `true` si el rename SÍ se aplicó y lo único que falló fue reapuntar
    /// los bindings del vault.
    pub vault_only: bool,
}

impl RenameError {
    /// Mensaje listo para mostrar, que no miente sobre el estado en disco.
    pub fn message(&self, new_name: &str) -> String {
        if self.vault_only {
            format!(
                "el MCP se renombró a '{}', pero no se pudieron reapuntar sus secretos \
                 vinculados: {}. Volvé a vincularlos desde Env & Secrets.",
                new_name, self.source
            )
        } else {
            self.source.to_string()
        }
    }
}
