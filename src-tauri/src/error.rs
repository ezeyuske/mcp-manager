use serde::Serialize;
use thiserror::Error;

/// Errores propios de los adapters de configuración de apps externas.
///
/// Serializable hacia el frontend como `String` (ver `impl Serialize`).
/// Nunca debe originarse en un `unwrap`/`expect` sobre I/O: siempre se
/// construye a partir de un `Result` ya manejado.
#[derive(Debug, Error)]
pub enum AdapterError {
    #[error("no se pudo leer el archivo de configuración en {path}: {source}")]
    Io {
        path: String,
        #[source]
        source: std::io::Error,
    },

    #[error("el archivo de configuración en {path} tiene un JSON inválido: {message}")]
    InvalidJson { path: String, message: String },
}

/// El frontend solo necesita un mensaje legible, no la estructura interna
/// del error. Serializamos como el `Display` del error.
impl Serialize for AdapterError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

/// Errores de las rutas de ESCRITURA de configs externos (Fase 3 en
/// adelante). Cada variante corresponde a un punto de falla concreto del
/// pipeline seguro de escritura: serializar -> validar -> backup ->
/// escritura atómica.
///
/// Nunca debe originarse en un `unwrap`/`expect` sobre I/O: siempre se
/// construye a partir de un `Result` ya manejado.
#[derive(Debug, Error)]
pub enum WriteError {
    #[error("error de E/S en {path}: {source}")]
    Io {
        path: String,
        #[source]
        source: std::io::Error,
    },

    #[error("no se pudo serializar el contenido a escribir: {message}")]
    Serialize { message: String },

    #[error("la validación del JSON a escribir en {path} falló: {message}")]
    Validation { path: String, message: String },

    #[error("no se pudo crear el backup de {path}: {source}")]
    Backup {
        path: String,
        #[source]
        source: std::io::Error,
    },

    #[error("operación no soportada: {message}")]
    NotSupported { message: String },

    #[error("no se encontró el destino de la mutación: {message}")]
    TargetNotFound { message: String },

    #[error("keychain del sistema no disponible o falló: {message}")]
    Keychain { message: String },
}

/// El frontend solo necesita un mensaje legible, no la estructura interna
/// del error.
impl Serialize for WriteError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl From<AdapterError> for WriteError {
    fn from(err: AdapterError) -> Self {
        match err {
            AdapterError::Io { path, source } => WriteError::Io { path, source },
            AdapterError::InvalidJson { path, message } => {
                WriteError::Validation { path, message }
            }
        }
    }
}
