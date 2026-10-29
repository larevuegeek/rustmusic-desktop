use crate::entity::playlist::playlist::TrackRefRow;
use crate::{entity::playlist::track_liked::TrackLiked, mapper::playlist::liked::track_liked_view::TrackLikedView};

pub struct TrackLikedRepository;

impl TrackLikedRepository {

    pub async fn insert_track_liked<'e, E>(
        exec: E,
        path: &str,
        profil_id: i64,
        library_cache_id: Option<i64>
    ) -> Result<TrackLiked, sqlx::Error>
    where
        E: sqlx::Executor<'e, Database = sqlx::Sqlite>
    {

        let result: TrackLiked = sqlx::query_as::<_, TrackLiked>(
            r#"
            INSERT INTO track_liked (
                path,
                profil_id,
                library_cache_id,
                created_at,
                source_id
            )
            VALUES (?1, ?2, ?3, CURRENT_TIMESTAMP, ?4)

            ON CONFLICT(path, profil_id)
            DO NOTHING

            RETURNING *
            "#
        )
        .bind(path)
        .bind(profil_id)
        .bind(library_cache_id)
        .bind(crate::core::remote::source_of(path))
        .fetch_one(exec)
        .await?;

        Ok(result)

    }


    /// `source` : celle à garder ; `None` : toutes les sources actives.
    pub async fn find_all_by_profile<'e, E>(
        exec: E,
        profil_id: i64,
        source: Option<i64>,
    ) -> Result<Vec<TrackLikedView>, sqlx::Error>
    where
        E: sqlx::Executor<'e, Database = sqlx::Sqlite>
    {
        let sql = format!(
            r#"
                SELECT
                tl.id,
                tl.created_at  AS liked_at,
                tl.path,
                lc.id          AS library_cache_id,
                lc.title,
                lc.artist,
                lc.album,
                lc.duration,
                lc.thumbnail_path,
                lc.bits_per_sample,
                lc.audio_format,
                lc.mime_type
                FROM track_liked tl
                LEFT JOIN library_cache lc ON lc.id = tl.library_cache_id
                WHERE profil_id = ?1 {}
                ORDER BY tl.created_at DESC
            "#,
            if source.is_some() { "AND tl.source_id = ?2".to_string() } else { format!("AND {}", crate::core::remote::enabled_source("tl.source_id")) }
        );
        let mut query = sqlx::query_as::<_, TrackLikedView>(sqlx::AssertSqlSafe(sql)).bind(profil_id);
        if let Some(s) = source {
            query = query.bind(s);
        }
        let results = query.fetch_all(exec).await?;

        Ok(results)
    }

    pub async fn remove_track_liked<'e, E>(
        exec: E,
        path: &str,
        profil_id: i64
    ) -> Result<u64, sqlx::Error>
    where
        E: sqlx::Executor<'e, Database = sqlx::Sqlite>
    {

        let done = sqlx::query(
                    r#"
                    DELETE FROM track_liked
                    WHERE path = ?1
                    AND profil_id = ?2
                    "#
                )
                .bind(path)
                .bind(profil_id)
                .execute(exec)
                .await?;

        Ok(done.rows_affected())
    }

    /// J'aime venu du serveur (source : le serveur lui-même) ; rend 1 s'il est nouveau.
    pub async fn like_from_server<'e, E>(exec: E, path: &str, profil_id: i64, server_id: i64) -> Result<u64, sqlx::Error>
    where
        E: sqlx::Executor<'e, Database = sqlx::Sqlite>,
    {
        sqlx::query(
            "INSERT INTO track_liked (path, profil_id, library_cache_id, source_id) VALUES (?1, ?2, (SELECT id FROM library_cache WHERE path = ?1), ?3)
             ON CONFLICT(path, profil_id) DO NOTHING",
        )
        .bind(path)
        .bind(profil_id)
        .bind(server_id)
        .execute(exec)
        .await
        .map(|r| r.rows_affected())
    }

    /// Titres aimés du profil pour un export, chemin toujours présent (le fichier peut avoir disparu).
    pub async fn track_refs<'e, E>(exec: E, profil_id: i64) -> Result<Vec<TrackRefRow>, sqlx::Error>
    where
        E: sqlx::Executor<'e, Database = sqlx::Sqlite>,
    {
        sqlx::query_as(
            "SELECT tl.path, lc.title, lc.artist, lc.album, lc.duration
               FROM track_liked tl
          LEFT JOIN library_cache lc ON lc.id = tl.library_cache_id
              WHERE tl.profil_id = ?
           ORDER BY tl.created_at, tl.id",
        )
        .bind(profil_id)
        .fetch_all(exec)
        .await
    }

    /// Comme `insert_track_liked`, sans erreur si le titre est déjà aimé.
    pub async fn insert_if_absent<'e, E>(exec: E, path: &str, profil_id: i64, library_cache_id: Option<i64>) -> Result<(), sqlx::Error>
    where
        E: sqlx::Executor<'e, Database = sqlx::Sqlite>,
    {
        sqlx::query("INSERT OR IGNORE INTO track_liked (profil_id, path, library_cache_id, source_id) VALUES (?, ?, ?, ?)")
            .bind(profil_id)
            .bind(path)
            .bind(library_cache_id)
            .bind(crate::core::remote::source_of(path))
            .execute(exec)
            .await
            .map(|_| ())
    }

}
