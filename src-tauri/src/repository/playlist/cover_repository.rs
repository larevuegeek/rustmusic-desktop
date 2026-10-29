use sqlx::SqlitePool;

/// Pochette d'une piste : celle de son album, sinon sa vignette.
const COVER: &str = "COALESCE(NULLIF(la.cover_url, ''), lc.thumbnail_path)";

/// Filtre de source sur `alias.source_id` (un entier : sûr dans le texte de la requête).
fn source_filter(alias: &str, source_id: Option<i64>) -> String {
    match source_id {
        Some(s) => format!(" AND {alias}.source_id = {s}"),
        None => format!(" AND {}", crate::core::remote::enabled_source(&format!("{alias}.source_id"))),
    }
}

/// Pochettes des collections de la barre latérale : playlists, titres aimés, historique.
pub struct CoverRepository;

impl CoverRepository {
    /// Playlists du profil (hors mix) : (id, règles si elle est auto).
    pub async fn sidebar_playlists(pool: &SqlitePool, profil_id: i64, source_id: Option<i64>) -> Result<Vec<(i64, Option<String>)>, sqlx::Error> {
        sqlx::query_as(sqlx::AssertSqlSafe(format!(
            "SELECT id, CASE WHEN is_smart = 1 THEN rules END FROM playlists p WHERE profil_id = ? AND is_mix = 0{}",
            source_filter("p", source_id)
        )))
        .bind(profil_id)
        .fetch_all(pool)
        .await
    }

    /// Pochettes des premiers morceaux de la playlist, dans son ordre.
    pub async fn of_playlist(pool: &SqlitePool, playlist_id: i64, limit: i64) -> Result<Vec<String>, sqlx::Error> {
        sqlx::query_scalar(sqlx::AssertSqlSafe(format!(
            "SELECT {COVER} FROM playlist_items pi
             JOIN library_tracks lt ON lt.id = pi.library_track_id
             LEFT JOIN library_albums la ON la.id = lt.library_album_id
             LEFT JOIN library_cache lc ON lc.id = lt.cache_id
             WHERE pi.playlist_id = ? AND {COVER} IS NOT NULL
             ORDER BY pi.sort_index, pi.id LIMIT {limit}"
        )))
        .bind(playlist_id)
        .fetch_all(pool)
        .await
    }

    /// Pochettes des derniers titres aimés.
    pub async fn of_liked(pool: &SqlitePool, profil_id: i64, source_id: Option<i64>, limit: i64) -> Result<Vec<String>, sqlx::Error> {
        sqlx::query_scalar(sqlx::AssertSqlSafe(format!(
            "SELECT {COVER} FROM track_liked tl
             JOIN library_files lf ON lf.path = tl.path
             JOIN library_tracks lt ON lt.file_id = lf.id
             LEFT JOIN library_albums la ON la.id = lt.library_album_id
             LEFT JOIN library_cache lc ON lc.id = lt.cache_id
             WHERE tl.profil_id = ? AND {COVER} IS NOT NULL{}
             ORDER BY tl.created_at DESC LIMIT {limit}",
            source_filter("tl", source_id)
        )))
        .bind(profil_id)
        .fetch_all(pool)
        .await
    }

    /// Pochettes des derniers morceaux écoutés.
    pub async fn of_recent(pool: &SqlitePool, profil_id: i64, source_id: Option<i64>, limit: i64) -> Result<Vec<String>, sqlx::Error> {
        sqlx::query_scalar(sqlx::AssertSqlSafe(format!(
            "SELECT {COVER} FROM recent_files rf
             JOIN library_files lf ON lf.path = rf.path
             JOIN library_tracks lt ON lt.file_id = lf.id
             LEFT JOIN library_albums la ON la.id = lt.library_album_id
             LEFT JOIN library_cache lc ON lc.id = lt.cache_id
             WHERE rf.profil_id = ? AND {COVER} IS NOT NULL{}
             ORDER BY rf.last_played_at DESC LIMIT {limit}",
            source_filter("rf", source_id)
        )))
        .bind(profil_id)
        .fetch_all(pool)
        .await
    }

    /// Pochette de chacune de ces pistes : (id de piste, pochette), ordre libre.
    pub async fn of_tracks(pool: &SqlitePool, ids: &[String]) -> Result<Vec<(String, String)>, sqlx::Error> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }
        let holes = vec!["?"; ids.len()].join(",");
        let sql = format!(
            "SELECT lt.id, {COVER} FROM library_tracks lt
             LEFT JOIN library_albums la ON la.id = lt.library_album_id
             LEFT JOIN library_cache lc ON lc.id = lt.cache_id
             WHERE lt.id IN ({holes}) AND {COVER} IS NOT NULL"
        );
        let mut query = sqlx::query_as::<_, (String, String)>(sqlx::AssertSqlSafe(sql));
        for id in ids {
            query = query.bind(id);
        }
        query.fetch_all(pool).await
    }
}
