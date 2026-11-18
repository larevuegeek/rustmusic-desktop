//! Compositeurs : la liste de l'onglet Artistes et les morceaux d'un compositeur.

use crate::app::State;

use crate::commands::library::library_command::{client_deezer, deezer_portrait, portraits_dir};
use crate::helper::library::thumbnail_helper::save_artist_image;
use crate::query::track::track_list_row::TrackListRow;
use crate::service::library::artist_split;
use crate::service::library::composer::{self, ComposerView};
use crate::app::platform::app_state::AppState;

pub async fn get_composers(state: State<'_, AppState>, library_id: i64) -> Result<Vec<ComposerView>, String> {
    composer::list(&state.pool, library_id).await.map_err(|e| format!("get_composers: {e}"))
}

pub async fn get_tracks_by_composer(state: State<'_, AppState>, library_id: i64, name: String) -> Result<Vec<TrackListRow>, String> {
    composer::tracks(&state.pool, library_id, &name).await.map_err(|e| format!("get_tracks_by_composer: {e}"))
}

/// Photo d'un compositeur pour sa page : déjà connue, sinon cherchée sur Deezer (une fois).
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
        Ok(Some(bytes)) => {
            let path_part = save_artist_image(&portraits_dir(), &composer::photo_file(&name), &bytes)?;
            composer::save_photo(&state.pool, &name, &path_part).await.map_err(|e| e.to_string())?;
            Ok(Some(path_part))
        }
        // Panne passagère : rien d'enregistré, la prochaine visite réessaiera.
        Err(e) => {
            log::warn!("[portraits] compositeur '{}' : {}", name, e);
            Ok(None)
        }
    }
}

/// De quoi découper un tag compositeur côté interface, exactement comme ici : mentions écartées et décisions des artistes multiples.
pub fn composer_split_config() -> serde_json::Value {
    serde_json::json!({ "junk": composer::JUNK, "rules": *artist_split::rules() })
}
