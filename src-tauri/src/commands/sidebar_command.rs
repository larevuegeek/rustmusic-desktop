use serde::Serialize;
use sqlx::SqlitePool;
use tauri::State;

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

const POCHETTE: &str = "COALESCE(NULLIF(la.cover_url, ''), lc.thumbnail_path)";

/// Les vignettes des playlists, des titres aimés et de l'historique.
#[tauri::command]
pub async fn get_sidebar_covers(
    state: State<'_, AppState>,
    profil_id: i64,
) -> Result<Vec<CollectionCovers>, String> {
    let pool = &state.pool;
    let mut sortie = Vec::new();

    let playlists: Vec<(i64, Option<String>)> = sqlx::query_as(
        "SELECT id, CASE WHEN is_smart = 1 THEN rules END FROM playlists WHERE profil_id = ? AND is_mix = 0",
    )
    .bind(profil_id)
    .fetch_all(pool)
    .await
    .map_err(|e| format!("Playlists : {e}"))?;

    for (id, regles) in playlists {
        let covers = match regles {
            Some(json) => pochettes_intelligentes(pool, id, &json).await.unwrap_or_default(),
            None => pochettes(pool, format!(
                "SELECT {POCHETTE} FROM playlist_items pi
                 JOIN library_tracks lt ON lt.id = pi.library_track_id
                 LEFT JOIN library_albums la ON la.id = lt.library_album_id
                 LEFT JOIN library_cache lc ON lc.id = lt.cache_id
                 WHERE pi.playlist_id = ? AND {POCHETTE} IS NOT NULL
                 ORDER BY pi.sort_index, pi.id LIMIT {PISTES_LUES}"
            ), Some(id)).await?,
        };
        sortie.push(CollectionCovers { key: format!("playlist:{id}"), covers });
    }

    let aimes = pochettes(pool, format!(
        "SELECT {POCHETTE} FROM track_liked tl
         JOIN library_files lf ON lf.path = tl.path
         JOIN library_tracks lt ON lt.file_id = lf.id
         LEFT JOIN library_albums la ON la.id = lt.library_album_id
         LEFT JOIN library_cache lc ON lc.id = lt.cache_id
         WHERE tl.profil_id = ? AND {POCHETTE} IS NOT NULL
         ORDER BY tl.created_at DESC LIMIT {PISTES_LUES}"
    ), Some(profil_id)).await?;
    sortie.push(CollectionCovers { key: "liked".into(), covers: aimes });

    // L'historique n'est pas rangé par profil.
    let recents = pochettes(pool, format!(
        "SELECT {POCHETTE} FROM recent_files rf
         JOIN library_files lf ON lf.path = rf.path
         JOIN library_tracks lt ON lt.file_id = lf.id
         LEFT JOIN library_albums la ON la.id = lt.library_album_id
         LEFT JOIN library_cache lc ON lc.id = lt.cache_id
         WHERE {POCHETTE} IS NOT NULL
         ORDER BY rf.last_played_at DESC LIMIT {PISTES_LUES}"
    ), None).await?;
    sortie.push(CollectionCovers { key: "recent".into(), covers: recents });

    Ok(sortie)
}

async fn pochettes(pool: &SqlitePool, sql: String, param: Option<i64>) -> Result<Vec<String>, String> {
    let mut q = sqlx::query_scalar::<_, String>(sqlx::AssertSqlSafe(sql));
    if let Some(p) = param {
        q = q.bind(p);
    }
    let lignes = q
        .fetch_all(pool)
        .await
        .map_err(|e| format!("Pochettes : {e}"))?;
    Ok(distinctes(lignes))
}

/// Le moteur de règles reste seul juge du contenu : on lui demande ses pistes.
async fn pochettes_intelligentes(pool: &SqlitePool, playlist_id: i64, json: &str) -> Result<Vec<String>, String> {
    let rules: crate::core::smart_playlist::rules::SmartRules =
        serde_json::from_str(json).map_err(|e| format!("Règles illisibles : {e}"))?;
    let pistes = crate::core::smart_playlist::engine::evaluate(pool, playlist_id, &rules).await?;
    let ids: Vec<String> = pistes.into_iter().take(PISTES_LUES as usize).map(|t| t.library_track_id).collect();
    if ids.is_empty() {
        return Ok(Vec::new());
    }

    let trous = std::iter::repeat_n("?", ids.len()).collect::<Vec<_>>().join(",");
    let sql = format!(
        "SELECT lt.id, {POCHETTE} FROM library_tracks lt
         LEFT JOIN library_albums la ON la.id = lt.library_album_id
         LEFT JOIN library_cache lc ON lc.id = lt.cache_id
         WHERE lt.id IN ({trous}) AND {POCHETTE} IS NOT NULL"
    );
    let mut q = sqlx::query_as::<_, (String, String)>(sqlx::AssertSqlSafe(sql));
    for id in &ids {
        q = q.bind(id);
    }
    let trouvees: std::collections::HashMap<String, String> = q
        .fetch_all(pool)
        .await
        .map_err(|e| format!("Pochettes : {e}"))?
        .into_iter()
        .collect();

    // Dans l'ordre de la playlist, pas dans celui de la base.
    Ok(distinctes(ids.iter().filter_map(|id| trouvees.get(id).cloned()).collect()))
}

fn distinctes(lignes: Vec<String>) -> Vec<String> {
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
