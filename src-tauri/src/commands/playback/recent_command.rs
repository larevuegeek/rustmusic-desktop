use std::path::Path;

use tauri::State;

use crate::{mapper::recent::recent_file_list_view::RecentFileListView, repository::recent::recent_file_repository::RecentFileRepository, state::AppState};

/// L'historique du profil (chaque profil a le sien).
#[tauri::command]
pub async fn get_recent_files(
    state: State<'_, AppState>,
    profil_id: i64,
    source_id: Option<i64>,
) -> Result<Vec<RecentFileListView>, String> {

    let recent_files: Vec<RecentFileListView> = match RecentFileRepository::get_recent_files(&state.pool, profil_id, source_id).await {
        Ok(recent_files) => recent_files,
        Err(e) => return Err(format!("Failed to get recent files : {}", e))
    };

    Ok(recent_files)
}


#[tauri::command]
pub async fn insert_recent_file(
    state: State<'_, AppState>,
    profil_id: i64,
    path: String,
) -> Result<(), String> {
    // Les épisodes ont leur propre suivi : l'historique reste musical.
    if crate::core::podcast::download::is_episode_path(Path::new(&path)) {
        return Ok(());
    }

    let _ = match RecentFileRepository::insert_recent_file(&state.pool, profil_id, path).await {
        Ok(library) => library,
        Err(e) => return Err(format!("Failed to insert recent file : {}", e))
    };

    Ok(())
}

#[tauri::command]
pub async fn remove_recent_file(
    state: State<'_, AppState>,
    profil_id: i64,
    path: String,
) -> Result<(), String> {

    let _ = match RecentFileRepository::remove_recent_file(&state.pool, profil_id, path).await {
        Ok(r) => r,
        Err(e) => return Err(format!("Failed to remove recent file : {}", e))
    };

    Ok(())
}

#[tauri::command]
pub async fn clear_recent_files(
    state: State<'_, AppState>,
    profil_id: i64,
    source_id: Option<i64>,
) -> Result<(), String> {

    let _ = match RecentFileRepository::clear_recent_files(&state.pool, profil_id, source_id).await {
        Ok(r) => r,
        Err(e) => return Err(format!("Failed to clear recent files : {}", e))
    };

    Ok(())
}

/// Mémorise où l'on en est dans le morceau en cours, pour pouvoir reprendre.
#[tauri::command]
pub async fn save_playback_position(
    state: State<'_, AppState>,
    profil_id: i64,
    path: String,
    position: f64,
    duration: Option<f64>,
) -> Result<(), String> {
    if crate::service::podcast::podcast_service::save_position(&state.pool, Path::new(&path), position, duration).await {
        return Ok(());
    }
    RecentFileRepository::save_position(&state.pool, profil_id, &path, position)
        .await
        .map_err(|e| format!("Position d'écoute : {e}"))
}
