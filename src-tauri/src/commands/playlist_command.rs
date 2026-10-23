use serde::Deserialize;
use tauri::State;

use crate::service::external_server::feedback_service::{self, Feedback};
use crate::service::external_server::playlist_service;

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
    /// Créée sur ce serveur (synchronisée avec lui), sinon locale.
    #[serde(default)]
    pub server_id: Option<i64>,
    /// Source de la playlist (0 : « Cet ordinateur ») ; celle du serveur s'il y en a un.
    #[serde(default)]
    pub source_id: Option<i64>,
}

#[derive(Deserialize)]
pub struct UpdatePlaylistPayload {
    pub id: i64,
    pub name: String,
    pub description: Option<String>,
    pub color: String,
    pub icon: String,
}

/// Playlist d'un serveur : le changement lui est renvoyé en arrière-plan.
async fn send_playlist(state: &AppState, playlist_id: i64, feedback: Feedback) {
    if let Some((server_id, remote_id)) = playlist_service::remote_of(&state.pool, playlist_id).await {
        let pool = state.pool.clone();
        tauri::async_runtime::spawn(async move { feedback_service::send_to(&pool, server_id, &remote_id, feedback).await });
    }
}

/// Morceau d'un serveur : l'action lui est renvoyée en arrière-plan.
pub(crate) fn send_feedback(state: &AppState, path: String, feedback: Feedback) {
    if !crate::service::external_server::track_service::is_remote(&path) {
        return;
    }
    let pool = state.pool.clone();
    tauri::async_runtime::spawn(async move { feedback_service::send(&pool, &path, feedback).await });
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
    send_feedback(&state, path, Feedback::Star(true));

    Ok(track_liked)
}

#[tauri::command]
pub async fn get_tracks_liked(
    state: State<'_, AppState>, 
    profil_id: i64,
    source_id: Option<i64>,
) -> Result<Vec<TrackLikedView>, String> {

    let tracks: Vec<TrackLikedView> = match TrackLikedRepository::find_all_by_profile(&state.pool, profil_id, source_id).await {
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
    send_feedback(&state, path, Feedback::Star(false));

    Ok(())
}

#[tauri::command]
pub async fn get_playlists(
    state: State<'_, AppState>,
    profil_id: i64,
    source_id: Option<i64>,
) -> Result<Vec<Playlist>, String> {

    let mut playlists: Vec<Playlist> = match PlaylistRepository::find_all_by_profile_id(&state.pool, profil_id, source_id).await {
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
        if let Ok(n) = crate::core::smart_playlist::engine::count(&state.pool, &regles, Some(p.source_id)).await {
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

    // Sur un serveur : créée là-bas d'abord (il doit répondre), rattachée ensuite.
    let remote = match payload.server_id {
        Some(server_id) => Some((server_id, playlist_service::create_on_server(&state.pool, server_id, payload.name.trim()).await?)),
        None => None,
    };
    let has_description = payload.description.as_deref().is_some_and(|d| !d.trim().is_empty());

    let mut playlist: Playlist = match PlaylistRepository::insert_playlist(
        &state.pool,
        payload.profil_id,
        None,
        payload.name,
        payload.description,
        payload.color,
        payload.icon,
        None,
        0,
        payload.server_id.or(payload.source_id).unwrap_or(0),
    ).await {
        Ok(playlist) => playlist,
        Err(e) => return Err(format!("Failed to create playlist : {}", e))
    };

    if let Some((server_id, remote_id)) = remote {
        playlist_service::attach(&state.pool, playlist.id, server_id, &remote_id).await.map_err(|e| e.to_string())?;
        playlist.server_id = Some(server_id);
        if has_description {
            send_playlist(&state, playlist.id, Feedback::PlaylistInfo(playlist.id)).await;
        }
    }

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
    if playlist.name != existing.name || playlist.description != existing.description {
        send_playlist(&state, playlist.id, Feedback::PlaylistInfo(playlist.id)).await;
    }

    Ok(playlist)
}

/// Nouvel ordre des playlists du profil (barre latérale), `ids` dans l'ordre voulu.
#[tauri::command]
pub async fn reorder_playlists(state: State<'_, AppState>, profil_id: i64, ids: Vec<i64>) -> Result<(), String> {
    let mut tx = state.pool.begin().await.map_err(|e| e.to_string())?;
    for (position, id) in ids.iter().enumerate() {
        sqlx::query("UPDATE playlists SET position = ? WHERE id = ? AND profil_id = ?")
            .bind(position as i64)
            .bind(id)
            .bind(profil_id)
            .execute(&mut *tx)
            .await
            .map_err(|e| format!("reorder_playlists: {e}"))?;
    }
    tx.commit().await.map_err(|e| e.to_string())
}

/// Épingle la playlist dans la barre latérale, ou l'en retire.
#[tauri::command]
pub async fn set_playlist_pinned(
    state: State<'_, AppState>,
    playlist_id: i64,
    pinned: bool,
) -> Result<(), String> {
    let r = sqlx::query("UPDATE playlists SET pinned = ? WHERE id = ? AND is_mix = 0")
        .bind(pinned)
        .bind(playlist_id)
        .execute(&state.pool)
        .await
        .map_err(|e| format!("set_playlist_pinned: {e}"))?;
    if r.rows_affected() == 0 {
        return Err(format!("set_playlist_pinned: playlist {playlist_id} introuvable"));
    }
    Ok(())
}

#[tauri::command]
pub async fn delete_playlist(
    state: State<'_, AppState>,
    playlist_id: i64,
) -> Result<(), String> {

    let remote = playlist_service::remote_of(&state.pool, playlist_id).await;
    if let Err(e) = PlaylistRepository::delete_playlist(&state.pool, playlist_id).await {
        log::warn!("delete_playlist failed: {}", e);
    }
    if let Some((server_id, remote_id)) = remote {
        let pool = state.pool.clone();
        tauri::async_runtime::spawn(async move { feedback_service::send_to(&pool, server_id, &remote_id, Feedback::PlaylistDelete).await });
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
        let source = PlaylistRepository::source_of(&state.pool, playlist_id).await.map_err(|e| e.to_string())?;
        return crate::core::smart_playlist::engine::evaluate(&state.pool, playlist_id, &rules, Some(source))
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

    // Une playlist ne prend que les morceaux de sa source (celle de leur bibliothèque).
    let track_source: Option<i64> = sqlx::query_scalar("SELECT COALESCE(l.server_id, 0) FROM library_tracks lt JOIN library l ON l.id = lt.library_id WHERE lt.id = ?")
        .bind(&track_id)
        .fetch_optional(&state.pool)
        .await
        .map_err(|e| e.to_string())?;
    let source = PlaylistRepository::source_of(&state.pool, playlist_id).await.map_err(|e| e.to_string())?;
    if track_source != Some(source) {
        return Err("server_playlist_foreign_track".into());
    }

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
    send_playlist(&state, playlist_id, Feedback::PlaylistSongs(playlist_id)).await;

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
        send_playlist(&state, item.playlist_id, Feedback::PlaylistSongs(item.playlist_id)).await;
    }

    Ok(())
}