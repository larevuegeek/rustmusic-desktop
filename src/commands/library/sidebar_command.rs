use serde::Serialize;
use sqlx::SqlitePool;
use crate::app::State;

use crate::repository::playlist::cover_repository::CoverRepository;
use crate::app::platform::app_state::AppState;

/// Jusqu'à quatre pochettes distinctes d'une collection, pour sa vignette.
#[derive(Debug, Clone, Serialize)]
pub struct CollectionCovers {
    /// `playlist:<id>`, `liked` ou `recent`.
    pub key: String,
    pub covers: Vec<String>,
}

const MAX_COVERS: usize = 4;
// Assez de pistes pour trouver quatre albums différents sans tout relire.
const TRACKS_READ: i64 = 200;

/// Les vignettes des playlists, des titres aimés et de l'historique.
pub async fn get_sidebar_covers(
    state: State<'_, AppState>,
    profile_id: i64,
    source_id: Option<i64>,
) -> Result<Vec<CollectionCovers>, String> {
    let pool = &state.pool;
    let mut output = Vec::new();
    let covers_list = |r: Result<Vec<String>, sqlx::Error>| r.map(distinct).map_err(|e| format!("Pochettes : {e}"));

    let playlists = CoverRepository::sidebar_playlists(pool, profile_id, source_id).await.map_err(|e| format!("Playlists : {e}"))?;
    for (id, rules) in playlists {
        let covers = match rules {
            Some(json) => smart_covers(pool, id, &json).await.unwrap_or_default(),
            None => covers_list(CoverRepository::of_playlist(pool, id, TRACKS_READ).await)?,
        };
        output.push(CollectionCovers { key: format!("playlist:{id}"), covers });
    }

    let liked_tracks = covers_list(CoverRepository::of_liked(pool, profile_id, source_id, TRACKS_READ).await)?;
    output.push(CollectionCovers { key: "liked".into(), covers: liked_tracks });

    let recent = covers_list(CoverRepository::of_recent(pool, profile_id, source_id, TRACKS_READ).await)?;
    output.push(CollectionCovers { key: "recent".into(), covers: recent });

    Ok(output)
}

/// Le moteur de règles reste seul juge du contenu : on lui demande ses pistes.
async fn smart_covers(pool: &SqlitePool, playlist_id: i64, json: &str) -> Result<Vec<String>, String> {
    let rules: crate::core::smart_playlist::rules::SmartRules =
        serde_json::from_str(json).map_err(|e| format!("Règles illisibles : {e}"))?;
    let source = crate::repository::playlist::playlist_repository::PlaylistRepository::source_of(pool, playlist_id).await.map_err(|e| e.to_string())?;
    let tracks_list = crate::core::smart_playlist::engine::evaluate(pool, playlist_id, &rules, Some(source)).await?;
    let ids: Vec<String> = tracks_list.into_iter().take(TRACKS_READ as usize).map(|t| t.library_track_id).collect();
    if ids.is_empty() {
        return Ok(Vec::new());
    }

    let found_items: std::collections::HashMap<String, String> = CoverRepository::of_tracks(pool, &ids)
        .await
        .map_err(|e| format!("Pochettes : {e}"))?
        .into_iter()
        .collect();

    // Dans l'ordre de la playlist, pas dans celui de la base.
    Ok(distinct(ids.iter().filter_map(|id| found_items.get(id).cloned()).collect()))
}

fn distinct(lines: Vec<String>) -> Vec<String> {
    let mut views = Vec::new();
    for p in lines {
        if !views.contains(&p) {
            views.push(p);
            if views.len() == MAX_COVERS {
                break;
            }
        }
    }
    views
}
