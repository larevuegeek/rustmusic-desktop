//! Commandes du suivi d'écoute (ListenBrainz).

use std::path::Path;

use tauri::State;

use crate::service::scrobble::scrobble_service::{self, ScrobbleStatus};
use crate::state::AppState;

fn db(e: sqlx::Error) -> String {
    e.to_string()
}

#[tauri::command]
pub async fn listenbrainz_connect(state: State<'_, AppState>, profil_id: i64, token: String) -> Result<String, String> {
    scrobble_service::connect(&state.pool, profil_id, &token).await
}

#[tauri::command]
pub async fn listenbrainz_disconnect(state: State<'_, AppState>, profil_id: i64) -> Result<(), String> {
    scrobble_service::disconnect(&state.pool, profil_id).await.map_err(db)
}

#[tauri::command]
pub async fn listenbrainz_status(state: State<'_, AppState>, profil_id: i64) -> Result<Option<ScrobbleStatus>, String> {
    scrobble_service::status(&state.pool, profil_id).await.map_err(db)
}

#[tauri::command]
pub async fn listenbrainz_set_now_playing(state: State<'_, AppState>, profil_id: i64, on: bool) -> Result<(), String> {
    scrobble_service::set_now_playing(&state.pool, profil_id, on).await.map_err(db)
}

/// Une écoute comptée (moitié du morceau ou 4 min) ; `listened_at` : son début, en secondes Unix.
#[tauri::command]
pub async fn scrobble_listen(state: State<'_, AppState>, profil_id: i64, path: String, listened_at: i64) -> Result<(), String> {
    if crate::core::podcast::download::is_episode_path(Path::new(&path)) {
        return Ok(());
    }
    scrobble_service::listen(&state.pool, profil_id, &path, listened_at).await;
    Ok(())
}

/// Le morceau qui commence.
#[tauri::command]
pub async fn scrobble_now_playing(state: State<'_, AppState>, profil_id: i64, path: String) -> Result<(), String> {
    if crate::core::podcast::download::is_episode_path(Path::new(&path)) {
        return Ok(());
    }
    scrobble_service::now_playing(&state.pool, profil_id, &path).await;
    Ok(())
}

/// Au lancement : les écoutes restées en attente partent.
pub async fn init(state: &AppState) {
    scrobble_service::flush_all(&state.pool).await;
}
