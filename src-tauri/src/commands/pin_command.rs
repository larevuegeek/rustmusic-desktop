use tauri::State;

use crate::entity::library::library_pin::LibraryPinView;
use crate::repository::library::library_pin_repository::LibraryPinRepository;
use crate::state::AppState;

#[tauri::command]
pub async fn get_library_pins(
    state: State<'_, AppState>,
    library_id: i64,
) -> Result<Vec<LibraryPinView>, String> {
    LibraryPinRepository::find_by_library_id(&state.pool, library_id)
        .await
        .map_err(|e| format!("Failed to get library pins: {}", e))
}

#[tauri::command]
pub async fn pin_album(
    state: State<'_, AppState>,
    library_id: i64,
    album_id: String,
) -> Result<(), String> {
    // Sans ce contrôle, un id d'une autre bibliothèque s'épinglerait ici sans erreur.
    let present = LibraryPinRepository::album_in_library(&state.pool, library_id, &album_id)
        .await
        .map_err(|e| format!("Failed to check album: {}", e))?;
    if !present {
        return Err(format!("Album {} absent de la bibliothèque {}", album_id, library_id));
    }

    LibraryPinRepository::pin_album(&state.pool, library_id, &album_id)
        .await
        .map_err(|e| format!("Failed to pin album: {}", e))
}

#[tauri::command]
pub async fn unpin_album(
    state: State<'_, AppState>,
    library_id: i64,
    album_id: String,
) -> Result<(), String> {
    LibraryPinRepository::unpin_album(&state.pool, library_id, &album_id)
        .await
        .map_err(|e| format!("Failed to unpin album: {}", e))
}

#[tauri::command]
pub async fn pin_artist(
    state: State<'_, AppState>,
    library_id: i64,
    artist_id: String,
) -> Result<(), String> {
    // Les artistes sont communs à toutes les bibliothèques : on vérifie le lien.
    let present = LibraryPinRepository::artist_in_library(&state.pool, library_id, &artist_id)
        .await
        .map_err(|e| format!("Failed to check artist: {}", e))?;
    if !present {
        return Err(format!("Artiste {} absent de la bibliothèque {}", artist_id, library_id));
    }

    LibraryPinRepository::pin_artist(&state.pool, library_id, &artist_id)
        .await
        .map_err(|e| format!("Failed to pin artist: {}", e))
}

#[tauri::command]
pub async fn unpin_artist(
    state: State<'_, AppState>,
    library_id: i64,
    artist_id: String,
) -> Result<(), String> {
    LibraryPinRepository::unpin_artist(&state.pool, library_id, &artist_id)
        .await
        .map_err(|e| format!("Failed to unpin artist: {}", e))
}
