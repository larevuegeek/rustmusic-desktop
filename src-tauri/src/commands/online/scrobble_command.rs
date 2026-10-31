//! Commandes du suivi d'écoute (ListenBrainz, Last.fm).

use std::path::Path;

use tauri::State;

use crate::service::scrobble::scrobble_service::{self, ScrobbleStatus, LASTFM, LISTENBRAINZ};
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
    scrobble_service::disconnect(&state.pool, profil_id, LISTENBRAINZ).await.map_err(db)
}

#[tauri::command]
pub async fn listenbrainz_status(state: State<'_, AppState>, profil_id: i64) -> Result<Option<ScrobbleStatus>, String> {
    scrobble_service::status(&state.pool, profil_id, LISTENBRAINZ).await.map_err(db)
}

#[tauri::command]
pub async fn listenbrainz_set_now_playing(state: State<'_, AppState>, profil_id: i64, on: bool) -> Result<(), String> {
    scrobble_service::set_now_playing(&state.pool, profil_id, LISTENBRAINZ, on).await.map_err(db)
}

/// Cette compilation a-t-elle la clé d'API de RustMusic ? Sinon, chacun met la sienne.
#[tauri::command]
pub fn lastfm_has_builtin_key() -> bool {
    crate::core::scrobble::lastfm::builtin().is_some()
}

/// Rend la page Last.fm où autoriser RustMusic ; `api_key` et `api_secret` : la clé personnelle, facultative.
#[tauri::command]
pub async fn lastfm_begin(profil_id: i64, api_key: Option<String>, api_secret: Option<String>) -> Result<String, String> {
    scrobble_service::lastfm_begin(profil_id, api_key.zip(api_secret)).await
}

/// À rappeler tant qu'il rend `None` : l'utilisateur n'a pas encore autorisé.
#[tauri::command]
pub async fn lastfm_finish(state: State<'_, AppState>, profil_id: i64) -> Result<Option<String>, String> {
    scrobble_service::lastfm_finish(&state.pool, profil_id).await
}

#[tauri::command]
pub fn lastfm_cancel(profil_id: i64) {
    scrobble_service::lastfm_cancel(profil_id);
}

#[tauri::command]
pub async fn lastfm_disconnect(state: State<'_, AppState>, profil_id: i64) -> Result<(), String> {
    scrobble_service::disconnect(&state.pool, profil_id, LASTFM).await.map_err(db)
}

#[tauri::command]
pub async fn lastfm_status(state: State<'_, AppState>, profil_id: i64) -> Result<Option<ScrobbleStatus>, String> {
    scrobble_service::status(&state.pool, profil_id, LASTFM).await.map_err(db)
}

#[tauri::command]
pub async fn lastfm_set_now_playing(state: State<'_, AppState>, profil_id: i64, on: bool) -> Result<(), String> {
    scrobble_service::set_now_playing(&state.pool, profil_id, LASTFM, on).await.map_err(db)
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
