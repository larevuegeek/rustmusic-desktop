use serde::Serialize;
use sqlx::SqlitePool;
use tauri::State;

use crate::repository::playlist::cover_repository::CoverRepository;
use crate::state::AppState;

/// Jusqu'à quatre pochettes distinctes d'une collection, pour sa vignette.
#[derive(Debug, Clone, Serialize)]
pub struct CollectionCovers {
    /// `playlist:<id>`, `liked` ou `recent`.
    pub key: String,
    pub covers: Vec<String>,
}

const MAX_POCHETTES: usize = 4;
// Assez de pistes pour trouver quatre albums différents sans tout relire.
const PISTES_LUES: i64 = 200;

/// Les vignettes des playlists, des titres aimés et de l'historique.
#[tauri::command]
pub async fn get_sidebar_covers(
    state: State<'_, AppState>,
    profil_id: i64,
    source_id: Option<i64>,
) -> Result<Vec<CollectionCovers>, String> {
    let pool = &state.pool;
    let mut sortie = Vec::new();
    let pochettes = |r: Result<Vec<String>, sqlx::Error>| r.map(distinct).map_err(|e| format!("Pochettes : {e}"));

    let playlists = CoverRepository::sidebar_playlists(pool, profil_id, source_id).await.map_err(|e| format!("Playlists : {e}"))?;
    for (id, regles) in playlists {
        let covers = match regles {
            Some(json) => smart_covers(pool, id, &json).await.unwrap_or_default(),
            None => pochettes(CoverRepository::of_playlist(pool, id, PISTES_LUES).await)?,
        };
        sortie.push(CollectionCovers { key: format!("playlist:{id}"), covers });
    }

    let aimes = pochettes(CoverRepository::of_liked(pool, profil_id, source_id, PISTES_LUES).await)?;
    sortie.push(CollectionCovers { key: "liked".into(), covers: aimes });

    let recents = pochettes(CoverRepository::of_recent(pool, profil_id, source_id, PISTES_LUES).await)?;
    sortie.push(CollectionCovers { key: "recent".into(), covers: recents });

    Ok(sortie)
}

/// Le moteur de règles reste seul juge du contenu : on lui demande ses pistes.
async fn smart_covers(pool: &SqlitePool, playlist_id: i64, json: &str) -> Result<Vec<String>, String> {
    let rules: crate::core::smart_playlist::rules::SmartRules =
        serde_json::from_str(json).map_err(|e| format!("Règles illisibles : {e}"))?;
    let source = crate::repository::playlist::playlist_repository::PlaylistRepository::source_of(pool, playlist_id).await.map_err(|e| e.to_string())?;
    let pistes = crate::core::smart_playlist::engine::evaluate(pool, playlist_id, &rules, Some(source)).await?;
    let ids: Vec<String> = pistes.into_iter().take(PISTES_LUES as usize).map(|t| t.library_track_id).collect();
    if ids.is_empty() {
        return Ok(Vec::new());
    }

    let trouvees: std::collections::HashMap<String, String> = CoverRepository::of_tracks(pool, &ids)
        .await
        .map_err(|e| format!("Pochettes : {e}"))?
        .into_iter()
        .collect();

    // Dans l'ordre de la playlist, pas dans celui de la base.
    Ok(distinct(ids.iter().filter_map(|id| trouvees.get(id).cloned()).collect()))
}

fn distinct(lignes: Vec<String>) -> Vec<String> {
    let mut vues = Vec::new();
    for p in lignes {
        if !vues.contains(&p) {
            vues.push(p);
            if vues.len() == MAX_POCHETTES {
                break;
            }
        }
    }
    vues
}
