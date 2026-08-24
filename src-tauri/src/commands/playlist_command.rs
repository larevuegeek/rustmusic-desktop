use serde::Deserialize;
use tauri::State;

use crate::{
    entity::playlist::{playlist::Playlist, track_liked::TrackLiked},
    mapper::playlist::{liked::track_liked_view::TrackLikedView, playlist::playlist_track_view::PlaylistTrackView},
    repository::{
        library::{
            library_cache_repository::LibraryCacheRepository,
            library_files_repository::LibraryFilesRepository,
            library_track_repository::LibraryTrackRepository,
        },
        playlist::{
            playlist_item_repository::PlaylistItemRepository,
            playlist_repository::PlaylistRepository,
            track_liked_repository::TrackLikedRepository,
        },
    },
    state::AppState
};

#[derive(Deserialize)]
pub struct CreatePlaylistPayload {
    pub profil_id: i64,
    pub name: String,
    pub description: Option<String>,
    pub color: String,
    pub icon: String,
}

#[derive(Deserialize)]
pub struct UpdatePlaylistPayload {
    pub id: i64,
    pub name: String,
    pub description: Option<String>,
    pub color: String,
    pub icon: String,
}

#[tauri::command]
pub async fn add_track_liked(
    state: State<'_, AppState>, 
    path: String,
    profil_id: i64,
) -> Result<TrackLiked, String> {

    let library_cache_id: Option<i64> = LibraryCacheRepository::get_library_cache_id_by_path(&state.pool, &path)
            .await
            .map_err(|e| e.to_string())?;

    let track_liked: TrackLiked = match TrackLikedRepository::insert_track_liked(&state.pool, &path, profil_id, library_cache_id).await {
        Ok(library) => library,
        Err(e) => return Err(format!("Failed to insert track liked : {}", e))
    };

    Ok(track_liked)
}

#[tauri::command]
pub async fn get_tracks_liked(
    state: State<'_, AppState>, 
    profil_id: i64
) -> Result<Vec<TrackLikedView>, String> {

    let tracks: Vec<TrackLikedView> = match TrackLikedRepository::fin_all_by_profil(&state.pool, profil_id).await {
        Ok(tracks) => tracks,
        Err(e) => return Err(format!("Failed to get track liked : {}", e))
    };

    Ok(tracks)
}

#[tauri::command]
pub async fn remove_track_liked(
    state: State<'_, AppState>, 
    path: String,
    profil_id: i64,
) -> Result<(), String> {

    if let Err(e) = TrackLikedRepository::remove_track_liked(&state.pool, &path, profil_id).await {
        log::warn!("remove_track_liked failed: {}", e);
    }

    Ok(())
}

#[tauri::command]
pub async fn get_playlists(
    state: State<'_, AppState>,
    profil_id: i64,
) -> Result<Vec<Playlist>, String> {

    let mut playlists: Vec<Playlist> = match PlaylistRepository::find_all_by_profil_id(&state.pool, profil_id).await {
        Ok(playlists) => playlists,
        Err(e) => return Err(format!("Failed to get playlists : {}", e))
    };

    // ─── Le compte d'une playlist intelligente se recalcule ───
    //
    // Sa colonne `track_count` n'était mise à jour qu'à l'enregistrement de ses
    // règles. Or son contenu bouge tout seul : une playlist « les plus
    // écoutés » créée alors qu'aucun morceau n'avait d'écoute restait figée à
    // zéro dans la barre latérale, même après en avoir écouté dix. Le même
    // piège que le compteur de « Grunge », sous une autre forme — un chiffre
    // qui a l'air d'une donnée alors qu'il n'est qu'un souvenir.
    //
    // On paie une requête par playlist intelligente, à l'ouverture et au
    // rafraîchissement seulement. Mesuré sur quatorze mille pistes, chacune
    // tient dans la fraction de seconde ; afficher un chiffre faux coûte plus
    // cher.
    //
    // Un échec laisse la valeur enregistrée plutôt que d'effacer la liste : un
    // compteur approximatif vaut mieux qu'une barre latérale vide.
    for p in playlists.iter_mut().filter(|p| p.is_smart) {
        let Some(json) = PlaylistRepository::find_rules(&state.pool, p.id).await.ok().flatten()
        else {
            continue;
        };
        let Ok(regles) = serde_json::from_str(&json) else {
            continue;
        };
        if let Ok(n) = crate::core::smart_playlist::engine::count(&state.pool, &regles).await {
            p.track_count = n;
        }
    }

    Ok(playlists)
}

#[tauri::command]
pub async fn get_playlist(
    state: State<'_, AppState>,
    playlist_id: i64,
) -> Result<Playlist, String> {

    let playlist: Playlist = match PlaylistRepository::find_by_id(&state.pool, playlist_id).await {
        Ok(Some(playlist)) => playlist,
        Ok(None) => return Err(format!("Playlist not found")),
        Err(e) => return Err(format!("Failed to get playlist : {}", e))
    };

    Ok(playlist)
}

#[tauri::command]
pub async fn create_playlist(
    state: State<'_, AppState>,
    payload: CreatePlaylistPayload,
) -> Result<Playlist, String> {

    let playlist: Playlist = match PlaylistRepository::insert_playlist(
        &state.pool,
        payload.profil_id,
        None,
        payload.name,
        payload.description,
        payload.color,
        payload.icon,
        None,
        0,
    ).await {
        Ok(playlist) => playlist,
        Err(e) => return Err(format!("Failed to create playlist : {}", e))
    };

    Ok(playlist)
}

#[tauri::command]
pub async fn update_playlist(
    state: State<'_, AppState>,
    payload: UpdatePlaylistPayload,
) -> Result<Playlist, String> {

    let existing: Playlist = match PlaylistRepository::find_by_id(&state.pool, payload.id).await {
        Ok(Some(playlist)) => playlist,
        Ok(None) => return Err(format!("Playlist not found")),
        Err(e) => return Err(format!("Failed to get playlist : {}", e))
    };

    let playlist: Playlist = match PlaylistRepository::update_playlist(
        &state.pool,
        payload.id,
        payload.name,
        payload.description,
        payload.color,
        payload.icon,
        existing.cover,
        existing.library_id,
        existing.position,
    ).await {
        Ok(Some(playlist)) => playlist,
        Ok(None) => return Err(format!("Playlist not found")),
        Err(e) => return Err(format!("Failed to update playlist : {}", e))
    };

    Ok(playlist)
}

#[tauri::command]
pub async fn delete_playlist(
    state: State<'_, AppState>,
    playlist_id: i64,
) -> Result<(), String> {

    if let Err(e) = PlaylistRepository::delete_playlist(&state.pool, playlist_id).await {
        log::warn!("delete_playlist failed: {}", e);
    }

    Ok(())
}

#[tauri::command]
pub async fn get_playlist_tracks(
    state: State<'_, AppState>,
    playlist_id: i64,
) -> Result<Vec<PlaylistTrackView>, String> {

    // ─── Une playlist intelligente se calcule au lieu de se lire ───
    //
    // La bifurcation est ici, et nulle part ailleurs : la page, la lecture, la
    // mise en file et l'export interrogent tous cette commande. Elle rend la
    // même forme dans les deux cas, si bien que rien en aval n'a à connaître la
    // différence.
    let regles: Option<String> =
        sqlx::query_scalar("SELECT rules FROM playlists WHERE id = ? AND is_smart = 1")
            .bind(playlist_id)
            .fetch_optional(&state.pool)
            .await
            .map_err(|e| format!("Failed to read playlist : {e}"))?
            .flatten();

    if let Some(json) = regles {
        let rules: crate::core::smart_playlist::rules::SmartRules =
            serde_json::from_str(&json).map_err(|e| format!("Règles illisibles : {e}"))?;
        return crate::core::smart_playlist::engine::evaluate(&state.pool, playlist_id, &rules)
            .await;
    }

    let tracks: Vec<PlaylistTrackView> = match PlaylistItemRepository::find_all_tracks_by_playlist_id(&state.pool, playlist_id).await {
        Ok(tracks) => tracks,
        Err(e) => return Err(format!("Failed to get playlist tracks : {}", e))
    };

    Ok(tracks)
}

#[tauri::command]
pub async fn add_track_to_playlist(
    state: State<'_, AppState>,
    playlist_id: i64,
    library_track_id: Option<String>,
    path: Option<String>,
) -> Result<(), String> {

    // Résoudre le library_track_id : soit passé directement, soit trouvé via le path
    let track_id = if let Some(id) = library_track_id {
        id
    } else if let Some(file_path) = path {
        // Chercher le track par le chemin du fichier
        let file = LibraryFilesRepository::find_by_path_any(&state.pool, &file_path)
            .await
            .map_err(|e| format!("Failed to find file: {}", e))?
            .ok_or_else(|| "Fichier non trouvé en bibliothèque".to_string())?;

        let track = LibraryTrackRepository::find_by_file_id(&state.pool, &file.id)
            .await
            .map_err(|e| format!("Failed to find track: {}", e))?
            .ok_or_else(|| "Track non trouvé en bibliothèque".to_string())?;

        track.id
    } else {
        return Err("Il faut fournir library_track_id ou path".to_string());
    };

    let sort_index = PlaylistItemRepository::get_next_sort_index(&state.pool, playlist_id)
        .await
        .map_err(|e| format!("Failed to get next sort index : {}", e))?;

    PlaylistItemRepository::insert_playlist_item(&state.pool, playlist_id, track_id, sort_index)
        .await
        .map_err(|e| format!("Failed to add track to playlist : {}", e))?;

    // Recalculer track_count et duration de la playlist
    PlaylistRepository::recalculate_stats(&state.pool, playlist_id)
        .await
        .map_err(|e| format!("Failed to update playlist stats : {}", e))?;

    Ok(())
}

#[tauri::command]
pub async fn remove_track_from_playlist(
    state: State<'_, AppState>,
    playlist_item_id: i64,
) -> Result<(), String> {

    // Récupérer le playlist_id avant suppression pour recalculer les stats
    let item = PlaylistItemRepository::find_by_id(&state.pool, playlist_item_id)
        .await
        .map_err(|e| format!("Failed to find playlist item: {}", e))?;

    PlaylistItemRepository::delete_playlist_item(&state.pool, playlist_item_id)
        .await
        .map_err(|e| format!("Failed to remove track: {}", e))?;

    // Recalculer les stats
    if let Some(item) = item {
        if let Err(e) = PlaylistRepository::recalculate_stats(&state.pool, item.playlist_id).await {
            log::warn!("recalculate_stats failed: {}", e);
        }
    }

    Ok(())
}