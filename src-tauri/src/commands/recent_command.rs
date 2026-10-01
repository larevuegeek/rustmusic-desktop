use tauri::State;

use crate::{mapper::recent::recent_file_list_view::RecentFileListView, repository::recent::recent_file_repository::RecentFileRepository, state::AppState};

#[tauri::command]
pub async fn get_recent_files(
    state: State<'_, AppState>, 
) -> Result<Vec<RecentFileListView>, String> {

    let recent_files: Vec<RecentFileListView> = match RecentFileRepository::get_recent_files(&state.pool).await {
        Ok(recent_files) => recent_files,
        Err(e) => return Err(format!("Failed to get recent files : {}", e))
    };

    Ok(recent_files)
}


#[tauri::command]
pub async fn insert_recent_file(
    state: State<'_, AppState>, 
    path: String,
    library_id: Option<i64>
) -> Result<(), String> {

    let _ = match RecentFileRepository::insert_recent_file(&state.pool, path, library_id).await {
        Ok(library) => library,
        Err(e) => return Err(format!("Failed to insert recent file : {}", e))
    };

    Ok(())
}

#[tauri::command]
pub async fn remove_recent_file(
    state: State<'_, AppState>, 
    path: String,
) -> Result<(), String> {

    let _ = match RecentFileRepository::remove_recent_file(&state.pool, path).await {
        Ok(r) => r,
        Err(e) => return Err(format!("Failed to remove recent file : {}", e))
    };

    Ok(())
}

#[tauri::command]
pub async fn clear_recent_files(
    state: State<'_, AppState>
) -> Result<(), String> {

    let _ = match RecentFileRepository::clear_recent_files(&state.pool).await {
        Ok(r) => r,
        Err(e) => return Err(format!("Failed to clear recent files : {}", e))
    };

    Ok(())
}

/// Mémorise où l'on en est dans le morceau en cours, pour pouvoir reprendre.
#[tauri::command]
pub async fn save_playback_position(
    state: State<'_, AppState>,
    path: String,
    position: f64,
) -> Result<(), String> {
    RecentFileRepository::save_position(&state.pool, &path, position)
        .await
        .map_err(|e| format!("Position d'écoute : {e}"))
}
