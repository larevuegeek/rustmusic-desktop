use std::path::{Path, PathBuf};

use sqlx::SqlitePool;
use tauri::State;

use crate::repository::library::{
    library_dirs_repository::LibraryDirRepository,
    library_track_repository::LibraryTrackRepository,
};
use crate::{
    entity::queue::queue_track::QueueTrack, 
    mapper::queue::queue_state_view::QueueStateView, 
    repository::queue::{
        queue_state_repository::QueueStateRepository, 
        queue_track_repository::QueueTrackRepository
    }, 
    state::AppState
};

#[tauri::command]
pub async fn get_queue(
    state: State<'_, AppState>, 
    profil_id: i64
) -> Result<QueueStateView, String> {

    let mut queue_state_view: QueueStateView = QueueStateRepository::get_queue(&state.pool, profil_id)
            .await
            .map_err(|e| format!("Failed to get queue state : {}", e))?;

    // Réparée une fois puis enregistrée : les lectures suivantes ne repassent plus par là.
    if reparer_pistes(&state.pool, &mut queue_state_view.tracks).await {
        if let Err(e) = QueueTrackRepository::replace_all(&state.pool, profil_id, queue_state_view.tracks.clone()).await {
            eprintln!("[file] réparation non enregistrée : {e}");
        }
    }

    Ok(queue_state_view)
}

/// Une file lancée depuis la vue Dossiers avant son correctif porte des chemins `\\?\UNC\…`
/// et des noms de fichier en guise de titres : on revient à la forme des dossiers importés,
/// puis on complète titre, artiste, durée et pochette. Vrai si quelque chose a changé.
async fn reparer_pistes(pool: &SqlitePool, pistes: &mut [QueueTrack]) -> bool {
    const VERBATIM: &str = r"\\?\";
    let mut change = false;

    if pistes.iter().any(|p| p.path.starts_with(VERBATIM)) {
        let racines: Vec<(PathBuf, String)> = LibraryDirRepository::all_paths(pool)
            .await
            .unwrap_or_default()
            .into_iter()
            .filter_map(|d| std::fs::canonicalize(&d).ok().map(|c| (c, d)))
            .collect();
        for p in pistes.iter_mut().filter(|p| p.path.starts_with(VERBATIM)) {
            let chemin = PathBuf::from(&p.path);
            // Le dossier le plus profond l'emporte, comme dans la vue Dossiers.
            let meilleur = racines
                .iter()
                .filter_map(|(c, d)| chemin.strip_prefix(c).ok().map(|reste| (c.as_os_str().len(), d, reste)))
                .max_by_key(|(n, _, _)| *n);
            if let Some((_, d, reste)) = meilleur {
                p.path = PathBuf::from(d).join(reste).to_string_lossy().to_string();
                change = true;
            }
        }
    }

    // L'image « pas de CD » de l'interface ne compte pas comme une pochette.
    let sans_pochette = |c: &Option<String>| c.as_deref().is_none_or(|x| x.starts_with("/images/"));
    let incomplets: Vec<String> = pistes
        .iter()
        .filter(|p| sans_pochette(&p.cover) || p.artist.is_none() || p.duration.is_none())
        .map(|p| p.path.clone())
        .collect();
    if incomplets.is_empty() {
        return change;
    }
    let fiches = LibraryTrackRepository::find_fiches_by_paths(pool, None, &incomplets).await;
    for p in pistes.iter_mut() {
        let Some(f) = fiches.get(&p.path) else { continue };
        // Un titre égal au nom du fichier n'était qu'un repli.
        let nom_fichier = Path::new(&p.path).file_name().map(|n| n.to_string_lossy().to_string());
        let repli = p.title.is_empty() || p.title == "Inconnu" || nom_fichier.as_deref() == Some(p.title.as_str());
        if let (true, Some(t)) = (repli, &f.title) {
            if *t != p.title { p.title = t.clone(); change = true; }
        }
        if p.artist.is_none() && f.artist.is_some() { p.artist = f.artist.clone(); change = true; }
        if p.duration.is_none() && f.duration.is_some() { p.duration = f.duration; change = true; }
        if sans_pochette(&p.cover) && f.thumbnail_path.is_some() { p.cover = f.thumbnail_path.clone(); change = true; }
    }
    change
}

#[tauri::command]
pub async fn add_queue_track(
    state: State<'_, AppState>, 
    profil_id: i64,
    payload: QueueTrack
) -> Result<(), String> {

    QueueTrackRepository::add_track(&state.pool, profil_id, payload)
        .await
        .map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
pub async fn update_queue_state_index(
    state: State<'_, AppState>, 
    profil_id: i64,
    current_index: i32
) -> Result<(), String> {

    QueueStateRepository::update_current_index(&state.pool, profil_id, current_index)
        .await
        .map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
pub async fn update_queue_state_shuffled(
    state: State<'_, AppState>, 
    profil_id: i64,
    is_shuffled: bool
) -> Result<(), String> {

    QueueStateRepository::update_is_shuffled(&state.pool, profil_id, is_shuffled)
        .await
        .map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
pub async fn update_queue_state_repeat_mode(
    state: State<'_, AppState>, 
    profil_id: i64,
    repeat_mode: &str
) -> Result<(), String> {

    QueueStateRepository::update_repeat_mode(&state.pool, profil_id, repeat_mode)
        .await
        .map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
pub async fn remove_queue_track(
    state: State<'_, AppState>, 
    profil_id: i64,
    queue_id: String
) -> Result<(), String> {

    QueueTrackRepository::remove_track(&state.pool, profil_id, &queue_id)
        .await
        .map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
pub async fn replace_queue_tracks(
    state: State<'_, AppState>, 
    profil_id: i64,
    payload: Vec<QueueTrack>
) -> Result<(), String> {
    QueueTrackRepository::replace_all(&state.pool, profil_id, payload)
        .await
        .map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
pub async fn clear_queue(
    state: State<'_, AppState>, 
    profil_id: i64
) -> Result<(), String> {

    QueueStateRepository::clear_queue(&state.pool, profil_id)
        .await
        .map_err(|e| e.to_string())?;

    Ok(())
}