use crate::adapters::all_adapters;
use crate::domain::Inventory;

/// Recorre todos los adapters de apps soportadas y arma un inventario
/// unificado. Si un adapter falla al leer su config (JSON corrupto,
/// error de I/O), el error queda acotado a `AppInfo.error` de esa app
/// puntual y el resto del inventario se arma igual: nunca devolvemos
/// `Err` global por un problema de una sola app.
#[tauri::command]
pub fn get_inventory() -> Result<Inventory, String> {
    let mut apps = Vec::new();
    let mut installations = Vec::new();

    for adapter in all_adapters() {
        let mut info = adapter.detect();

        if info.installed {
            match adapter.read() {
                Ok(mut found) => installations.append(&mut found),
                Err(err) => info.error = Some(err.to_string()),
            }
        }

        apps.push(info);
    }

    Ok(Inventory {
        apps,
        installations,
    })
}
