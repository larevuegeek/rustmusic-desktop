//! Ce que permet l'installation : Flatpak ou non, sortie exclusive possible ou non.

use crate::core::sandbox::{self, PlatformFeatures};

#[tauri::command]
pub fn get_platform_features() -> PlatformFeatures {
    sandbox::features()
}

/// Lancement au démarrage en Flatpak, par le portail du système.
#[tauri::command]
pub async fn set_autostart_portal(enabled: bool) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || sandbox::request_autostart(enabled))
        .await
        .map_err(|e| e.to_string())?
}
