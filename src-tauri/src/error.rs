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
