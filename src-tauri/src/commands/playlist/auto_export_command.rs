//! Export automatique des playlists : la passe (réglages, interface) et l'option d'une playlist.

use tauri::State;

use crate::service::playlist::auto_export::{self, ExportReport};
use crate::state::AppState;

/// Réécrit les playlists marquées qui ont changé. Appelée par l'interface après une modification,
/// un scan, au démarrage, et par « Exporter maintenant ».
#[tauri::command]
pub async fn playlist_auto_export_run(state: State<'_, AppState>) -> Result<ExportReport, String> {
    Ok(auto_export::run(&state.pool).await)
}

/// Active ou coupe l'option ; coupée, le fichier exporté est retiré.
#[tauri::command]
pub async fn playlist_set_auto_export(state: State<'_, AppState>, playlist_id: i64, enabled: bool) -> Result<ExportReport, String> {
    if !enabled {
        auto_export::forget(&state.pool, playlist_id).await;
    }
    sqlx::query("UPDATE playlists SET auto_export = ? WHERE id = ? AND server_id IS NULL")
        .bind(enabled)
        .bind(playlist_id)
        .execute(&state.pool)
        .await
        .map_err(|e| e.to_string())?;
    Ok(auto_export::run(&state.pool).await)
}
