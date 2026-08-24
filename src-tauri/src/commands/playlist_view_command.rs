//! Les pistes d'une playlist, au format de la bibliothèque.
//!
//! # Pourquoi une seconde forme
//! La vue tableau des onglets Morceaux, Albums, Artistes et Genres se compose à
//! partir de `TrackListView` : elle y puise les colonnes que l'utilisateur a
//! choisies, tags compris. `PlaylistTrackView`, lui, ne porte qu'une douzaine
//! de champs — de quoi afficher une ligne, pas de quoi trier par débit ou
//! afficher un label de disque.
//!
//! Plutôt que de gonfler `PlaylistTrackView` — et de faire payer ces colonnes à
//! tout ce qui s'en sert déjà, y compris la file d'attente — on rend la même
//! liste sous l'autre forme, et seule la vue tableau la demande.

use tauri::State;

use crate::mapper::library::track::track_list_item_view::{
    TrackListView, TRACK_VIEW_FROM, TRACK_VIEW_SELECT,
};
use crate::state::AppState;

/// Les pistes d'une playlist, rangée ou intelligente.
#[tauri::command]
pub async fn get_playlist_tracks_view(
    state: State<'_, AppState>,
    playlist_id: i64,
) -> Result<Vec<TrackListView>, String> {
    // Une playlist intelligente calcule son contenu : on lui demande ses
    // identifiants, puis on les remplit. Deux requêtes plutôt qu'une, mais le
    // moteur de règles reste seul maître de la sélection — le dupliquer ici
    // ferait diverger les deux vues de la même playlist.
    let regles: Option<String> =
        sqlx::query_scalar("SELECT rules FROM playlists WHERE id = ? AND is_smart = 1")
            .bind(playlist_id)
            .fetch_optional(&state.pool)
            .await
            .map_err(|e| format!("Lecture de la playlist : {e}"))?
            .flatten();

    if let Some(json) = regles {
        let rules: crate::core::smart_playlist::rules::SmartRules =
            serde_json::from_str(&json).map_err(|e| format!("Règles illisibles : {e}"))?;

        let calculees =
            crate::core::smart_playlist::engine::evaluate(&state.pool, playlist_id, &rules).await?;

        let ids: Vec<String> = calculees.into_iter().map(|t| t.library_track_id).collect();
        return remplir_dans_l_ordre(&state.pool, &ids).await;
    }

    let sql = format!(
        "{TRACK_VIEW_SELECT}
         {TRACK_VIEW_FROM}
         INNER JOIN playlist_items pi ON pi.library_track_id = lt.id
         WHERE pi.playlist_id = ?
         ORDER BY pi.sort_index ASC, pi.id ASC"
    );

    sqlx::query_as::<_, TrackListView>(&sql)
        .bind(playlist_id)
        .fetch_all(&state.pool)
        .await
        .map_err(|e| format!("Pistes de la playlist : {e}"))
}

/// Les pistes correspondant à une liste de chemins, dans l'ordre donné.
///
/// Sert les titres aimés et l'historique, qui désignent leurs morceaux par
/// chemin. Un fichier absent de la bibliothèque n'a pas de ligne à rendre : il
/// disparaît simplement de la vue tableau, qui n'a de toute façon aucune de ses
/// colonnes à afficher.
#[tauri::command]
pub async fn get_tracks_view_by_paths(
    state: State<'_, AppState>,
    paths: Vec<String>,
) -> Result<Vec<TrackListView>, String> {
    if paths.is_empty() {
        return Ok(Vec::new());
    }

    // Un `?` par chemin : les valeurs restent liées, et la longueur de la liste
    // ne change rien à la sécurité de la requête.
    let trous = std::iter::repeat_n("?", paths.len())
        .collect::<Vec<_>>()
        .join(",");
    let sql = format!("{TRACK_VIEW_SELECT}\n{TRACK_VIEW_FROM}\nWHERE lf.path IN ({trous})");

    let mut q = sqlx::query_as::<_, TrackListView>(&sql);
    for p in &paths {
        q = q.bind(p);
    }

    let trouvees = q
        .fetch_all(&state.pool)
        .await
        .map_err(|e| format!("Pistes par chemin : {e}"))?;

    Ok(ranger_selon(trouvees, &paths, |t| t.path.clone()))
}

/// Remplit des identifiants en conservant l'ordre demandé.
async fn remplir_dans_l_ordre(
    pool: &sqlx::SqlitePool,
    ids: &[String],
) -> Result<Vec<TrackListView>, String> {
    if ids.is_empty() {
        return Ok(Vec::new());
    }

    let trous = std::iter::repeat_n("?", ids.len())
        .collect::<Vec<_>>()
        .join(",");
    let sql = format!("{TRACK_VIEW_SELECT}\n{TRACK_VIEW_FROM}\nWHERE lt.id IN ({trous})");

    let mut q = sqlx::query_as::<_, TrackListView>(&sql);
    for id in ids {
        q = q.bind(id);
    }

    let trouvees = q
        .fetch_all(pool)
        .await
        .map_err(|e| format!("Pistes de la playlist : {e}"))?;

    Ok(ranger_selon(trouvees, ids, |t| t.id.clone()))
}

/// Remet une liste dans l'ordre d'une autre.
///
/// `IN (…)` ne promet aucun ordre : sans ce reclassement, une playlist
/// intelligente triée par écoutes ressortirait dans l'ordre où la base a trouvé
/// les lignes, et le tri demandé serait perdu en chemin.
fn ranger_selon<F>(mut trouvees: Vec<TrackListView>, ordre: &[String], cle: F) -> Vec<TrackListView>
where
    F: Fn(&TrackListView) -> String,
{
    let rang: std::collections::HashMap<&str, usize> = ordre
        .iter()
        .enumerate()
        .map(|(i, v)| (v.as_str(), i))
        .collect();

    // Les lignes sans rang connu — cas qui ne devrait pas se produire — vont
    // en fin plutôt que de faire échouer le classement.
    trouvees.sort_by_key(|t| rang.get(cle(t).as_str()).copied().unwrap_or(usize::MAX));
    trouvees
}
