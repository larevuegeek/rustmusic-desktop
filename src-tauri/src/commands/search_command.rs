use tauri::State;
use crate::entity::search::search_result::SearchResult;
use crate::repository::artist::artist_repository::ArtistRepository;
use crate::repository::library::library_album_repository::LibraryAlbumRepository;
use crate::repository::library::library_track_repository::LibraryTrackRepository;
use crate::state::AppState;

#[tauri::command]
pub async fn search(
    state: State<'_, AppState>,
    query: String,
    limit: Option<i64>,
    // Le profil (chacun ne voit que ses bibliothèques), puis la source ; `None` : toutes.
    profil_id: i64,
    source_id: Option<i64>,
) -> Result<Vec<SearchResult>, String> {
    let limit = limit.unwrap_or(10);
    let search_term = format!("%{}%", query.to_lowercase());

    let track_limit = (limit + 1) / 2;
    let album_limit = (limit * 3 / 10).max(1);
    let artist_limit = (limit * 2 / 10).max(1);

    let tracks = LibraryTrackRepository::search(&state.pool, &search_term, track_limit, profil_id, source_id)
        .await
        .map_err(|e| format!("Failed to search tracks: {}", e))?;

    let albums = LibraryAlbumRepository::search(&state.pool, &search_term, album_limit, profil_id, source_id)
        .await
        .map_err(|e| format!("Failed to search albums: {}", e))?;

    let artists = ArtistRepository::search(&state.pool, &search_term, artist_limit, profil_id, source_id)
        .await
        .map_err(|e| format!("Failed to search artists: {}", e))?;

    let mut results: Vec<SearchResult> = Vec::new();
    results.extend(tracks);
    results.extend(albums);
    results.extend(artists);

    Ok(results)
}
