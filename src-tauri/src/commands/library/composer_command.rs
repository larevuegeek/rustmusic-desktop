//! Compositeurs : la liste de l'onglet Artistes et les morceaux d'un compositeur.

use tauri::State;

use crate::commands::library::library_command::{client_deezer, deezer_portrait, portraits_dir};
use crate::helper::library::thumbnail_helper::save_artist_image;
use crate::mapper::library::track::track_list_item_view::TrackListView;
use crate::service::library::artist_split;
use crate::service::library::composer::{self, ComposerView};
use crate::state::AppState;

#[tauri::command]
pub async fn get_composers(state: State<'_, AppState>, library_id: i64) -> Result<Vec<ComposerView>, String> {
    composer::list(&state.pool, library_id).await.map_err(|e| format!("get_composers: {e}"))
}

#[tauri::command]
pub async fn get_tracks_by_composer(state: State<'_, AppState>, library_id: i64, name: String) -> Result<Vec<TrackListView>, String> {
    composer::tracks(&state.pool, library_id, &name).await.map_err(|e| format!("get_tracks_by_composer: {e}"))
}

/// Photo d'un compositeur pour sa page : déjà connue, sinon cherchée sur Deezer (une fois).
#[tauri::command]
pub async fn fetch_composer_image(state: State<'_, AppState>, name: String) -> Result<Option<String>, String> {
    match composer::known_photo(&state.pool, &name).await {
        Some(p) if !p.is_empty() => return Ok(Some(p)),
        Some(_) => return Ok(None),
        None => {}
    }
    let client = client_deezer(3)?;
    match deezer_portrait(&client, &name).await {
        Ok(None) => {
            composer::save_photo(&state.pool, &name, "").await.map_err(|e| e.to_string())?;
            Ok(None)
        }
        Ok(Some(octets)) => {
            let chemin = save_artist_image(&portraits_dir(), &composer::photo_file(&name), &octets)?;
            composer::save_photo(&state.pool, &name, &chemin).await.map_err(|e| e.to_string())?;
            Ok(Some(chemin))
        }
        // Panne passagère : rien d'enregistré, la prochaine visite réessaiera.
        Err(e) => {
            log::warn!("[portraits] compositeur '{}' : {}", name, e);
            Ok(None)
        }
    }
}

/// De quoi découper un tag compositeur côté interface, exactement comme ici : mentions écartées et décisions des artistes multiples.
#[tauri::command]
pub fn composer_split_config() -> serde_json::Value {
    serde_json::json!({ "junk": composer::JUNK, "rules": *artist_split::rules() })
}
