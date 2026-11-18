//! Les pistes d'une playlist, au format de la bibliothèque.
//!
//! # Pourquoi une seconde forme
//! La vue tableau des onglets Morceaux, Albums, Artistes et Genres se compose à
//! partir de `TrackListRow` : elle y puise les colonnes que l'utilisateur a
//! choisies, tags compris. `PlaylistTrackRow`, lui, ne porte qu'une douzaine
//! de champs — de quoi afficher une ligne, pas de quoi trier par débit ou
//! afficher un label de disque.
//!
//! Plutôt que de gonfler `PlaylistTrackRow` — et de faire payer ces colonnes à
//! tout ce qui s'en sert déjà, y compris la file d'attente — on rend la même
//! liste sous l'autre forme, et seule la vue tableau la demande.

use crate::app::State;

use crate::query::track::track_list_row::TrackListRow;
use crate::repository::library::library_track_repository::LibraryTrackRepository;
use crate::repository::playlist::playlist_item_repository::PlaylistItemRepository;
use crate::repository::playlist::playlist_repository::PlaylistRepository;
use crate::app::platform::app_state::AppState;

/// Les pistes d'une playlist, rangée ou intelligente.
pub async fn get_playlist_tracks_view(
    state: State<'_, AppState>,
    playlist_id: i64,
) -> Result<Vec<TrackListRow>, String> {
    // Une playlist intelligente calcule son contenu : on lui demande ses
    // identifiants, puis on les remplit. Deux requêtes plutôt qu'une, mais le
    // moteur de règles reste seul maître de la sélection — le dupliquer ici
    // ferait diverger les deux vues de la même playlist.
    let rules = PlaylistRepository::find_rules_and_mix(&state.pool, playlist_id)
        .await
        .map_err(|e| format!("Lecture de la playlist : {e}"))?;

    if let Some((Some(json), is_mix)) = rules {
        let mut rules: crate::core::smart_playlist::rules::SmartRules =
            serde_json::from_str(&json).map_err(|e| format!("Règles illisibles : {e}"))?;

        // Un mix tire large, puis garde de la variété : quelques titres par artiste.
        let size = rules.limit.as_ref().and_then(|l| l.count).unwrap_or(50).max(1);
        if is_mix {
            if let Some(l) = rules.limit.as_mut() {
                l.count = Some((size * 6).min(1200));
            }
        }

        let computed =
            crate::core::smart_playlist::engine::evaluate(&state.pool, playlist_id, &rules, Some(crate::repository::playlist::playlist_repository::PlaylistRepository::source_of(&state.pool, playlist_id).await.map_err(|e| e.to_string())?)).await?;

        let ids: Vec<String> = computed.into_iter().map(|t| t.library_track_id).collect();
        let tracks_list = fill_in_order(&state.pool, &ids).await?;
        return Ok(if is_mix {
            crate::core::smart_playlist::mix_variety::diversify(tracks_list, size as usize, |p| p.artist_id.clone())
        } else {
            tracks_list
        });
    }

    PlaylistItemRepository::find_track_views(&state.pool, playlist_id)
        .await
        .map_err(|e| format!("Pistes de la playlist : {e}"))
}

/// Les pistes correspondant à une liste de chemins, dans l'ordre donné.
///
/// Sert les titres aimés et l'historique, qui désignent leurs morceaux par
/// chemin. Un fichier absent de la bibliothèque n'a pas de ligne à rendre : il
/// disparaît simplement de la vue tableau, qui n'a de toute façon aucune de ses
/// colonnes à afficher.
pub async fn get_tracks_view_by_paths(
    state: State<'_, AppState>,
    paths: Vec<String>,
) -> Result<Vec<TrackListRow>, String> {
    if paths.is_empty() {
        return Ok(Vec::new());
    }

    let found_items = LibraryTrackRepository::find_views_by_paths(&state.pool, &paths)
        .await
        .map_err(|e| format!("Pistes par chemin : {e}"))?;

    Ok(arrange_by(found_items, &paths, |t| t.path.clone()))
}

/// Remplit des identifiants en conservant l'ordre demandé.
async fn fill_in_order(
    pool: &sqlx::SqlitePool,
    ids: &[String],
) -> Result<Vec<TrackListRow>, String> {
    if ids.is_empty() {
        return Ok(Vec::new());
    }

    let found_items = LibraryTrackRepository::find_views_by_ids(pool, ids)
        .await
        .map_err(|e| format!("Pistes de la playlist : {e}"))?;

    Ok(arrange_by(found_items, ids, |t| t.id.clone()))
}

/// Remet une liste dans l'ordre d'une autre.
///
/// `IN (…)` ne promet aucun ordre : sans ce reclassement, une playlist
/// intelligente triée par écoutes ressortirait dans l'ordre où la base a trouvé
/// les lignes, et le tri demandé serait perdu en chemin.
fn arrange_by<F>(mut found_items: Vec<TrackListRow>, order: &[String], key: F) -> Vec<TrackListRow>
where
    F: Fn(&TrackListRow) -> String,
{
    let rank: std::collections::HashMap<&str, usize> = order
        .iter()
        .enumerate()
        .map(|(i, v)| (v.as_str(), i))
        .collect();

    // Les lignes sans rang connu — cas qui ne devrait pas se produire — vont
    // en fin plutôt que de faire échouer le classement.
    found_items.sort_by_key(|t| rank.get(key(t).as_str()).copied().unwrap_or(usize::MAX));
    found_items
}
