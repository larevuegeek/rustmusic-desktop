use std::collections::HashMap;
use tauri::State;

use crate::{
    repository::settings::settings_repository::SettingsRepository,
    state::AppState
};

#[tauri::command]
pub async fn get_setting(
    state: State<'_, AppState>,
    key: String,
) -> Result<Option<String>, String> {
    SettingsRepository::get(&state.pool, &key)
        .await
        .map_err(|e| format!("Failed to get setting: {}", e))
}

#[tauri::command]
pub async fn set_setting(
    state: State<'_, AppState>,
    key: String,
    value: String,
) -> Result<(), String> {
    SettingsRepository::set(&state.pool, &key, &value)
        .await
        .map_err(|e| format!("Failed to set setting: {}", e))
}

#[tauri::command]
pub async fn get_all_settings(
    state: State<'_, AppState>,
) -> Result<HashMap<String, String>, String> {
    let rows = SettingsRepository::get_all(&state.pool)
        .await
        .map_err(|e| format!("Failed to get settings: {}", e))?;

    Ok(rows.into_iter().collect())
}

#[tauri::command]
pub async fn reset_application(
    state: State<'_, AppState>,
) -> Result<(), String> {
    // Vider toutes les tables dans l'ordre (respecter les FK)
    // Fermer le pool pour libérer le fichier DB
    state.pool.close().await;

    // Supprimer le fichier DB
    let mut db_dir = dirs::data_dir()
        .ok_or_else(|| "Failed to resolve data directory".to_string())?;
    db_dir.push("com.larevuegeek.rustmusic");
    let db_path = db_dir.join("rustmusic.db");

    if db_path.exists() {
        std::fs::remove_file(&db_path)
            .map_err(|e| format!("Failed to delete database: {}", e))?;
    }

    // Supprimer aussi les fichiers WAL/SHM de SQLite
    let wal_path = db_dir.join("rustmusic.db-wal");
    let shm_path = db_dir.join("rustmusic.db-shm");
    let _ = std::fs::remove_file(&wal_path);
    let _ = std::fs::remove_file(&shm_path);

    Ok(())
}
