use sqlx::SqlitePool;

pub struct LibraryStatsRepository;

impl LibraryStatsRepository {
    pub async fn get_main_counters(
        pool: &SqlitePool,
        library_id: i64,
    ) -> Result<(i64, f64, i64, f64, i64), sqlx::Error> {
        sqlx::query_as::<_, (i64, f64, i64, f64, i64)>(
            r#"SELECT
                COUNT(*) AS total_tracks,
                COALESCE(SUM(COALESCE(lt.duration, 0)), 0) AS total_duration,
                COALESCE(SUM(lf.size), 0) AS total_size,
                COALESCE(AVG(NULLIF(lt.bitrate, 0)), 0) AS avg_bitrate,
                COALESCE(SUM(lt.play_count), 0) AS total_play_count
            FROM library_tracks lt
            INNER JOIN library_files lf ON lf.id = lt.file_id
            WHERE lt.library_id = ?"#,
        )
        .bind(library_id)
        .fetch_one(pool)
        .await
    }

    pub async fn count_albums(pool: &SqlitePool, library_id: i64) -> i64 {
        sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM library_albums WHERE library_id = ?",
        )
        .bind(library_id)
        .fetch_one(pool)
        .await
        .unwrap_or(0)
    }

    /// Les artistes qui ont réellement quelque chose ici. `library_artists`
    /// seule en garde qui n'ont plus ni piste ni album.
    pub async fn count_artists(pool: &SqlitePool, library_id: i64) -> i64 {
        sqlx::query_scalar::<_, i64>(
            // Comptage ensembliste, voir `LibraryRepository`.
            r#"SELECT COUNT(*) FROM (
                   SELECT la.artist_id FROM library_artists la
                   WHERE la.library_id = ?1
                   INTERSECT
                   SELECT artist_id FROM (
                       SELECT lt.artist_id FROM library_tracks lt
                       WHERE lt.library_id = ?1 AND lt.artist_id IS NOT NULL
                       UNION
                       SELECT lta.artist_id FROM library_track_artists lta
                       WHERE lta.library_id = ?1
                       UNION
                       SELECT lb.artist_id FROM library_albums lb
                       WHERE lb.library_id = ?1 AND lb.artist_id IS NOT NULL
                   )
               )"#,
        )
        .bind(library_id)
        .fetch_one(pool)
        .await
        .unwrap_or(0)
    }

    pub async fn count_genres(pool: &SqlitePool, library_id: i64) -> i64 {
        sqlx::query_scalar::<_, i64>(
            r#"SELECT COUNT(DISTINCT lc.genre) FROM library_cache lc
               INNER JOIN library_tracks lt ON lt.cache_id = lc.id
               WHERE lt.library_id = ? AND lc.genre IS NOT NULL AND lc.genre != ''"#,
        )
        .bind(library_id)
        .fetch_one(pool)
        .await
        .unwrap_or(0)
    }

    pub async fn get_format_stats(
        pool: &SqlitePool,
        library_id: i64,
    ) -> Vec<(String, i64)> {
        sqlx::query_as::<_, (String, i64)>(
            r#"SELECT UPPER(COALESCE(lc.audio_format, lf.extension)) AS fmt, COUNT(*) AS cnt
               FROM library_tracks lt
               INNER JOIN library_files lf ON lf.id = lt.file_id
               LEFT JOIN library_cache lc ON lc.id = lt.cache_id
               WHERE lt.library_id = ?
               GROUP BY fmt ORDER BY cnt DESC"#,
        )
        .bind(library_id)
        .fetch_all(pool)
        .await
        .unwrap_or_default()
    }

    pub async fn count_quality_hires(pool: &SqlitePool, library_id: i64) -> i64 {
        sqlx::query_scalar::<_, i64>(
            r#"SELECT COUNT(*) FROM library_tracks lt
               LEFT JOIN library_cache lc ON lc.id = lt.cache_id
               WHERE lt.library_id = ? AND lc.bits_per_sample >= 24"#,
        )
        .bind(library_id)
        .fetch_one(pool)
        .await
        .unwrap_or(0)
    }

    pub async fn count_quality_lossless(pool: &SqlitePool, library_id: i64) -> i64 {
        sqlx::query_scalar::<_, i64>(
            r#"SELECT COUNT(*) FROM library_tracks lt
               LEFT JOIN library_cache lc ON lc.id = lt.cache_id
               WHERE lt.library_id = ? AND lc.bits_per_sample = 16
               AND LOWER(lc.audio_format) IN ('flac', 'wav', 'alac', 'aiff')"#,
        )
        .bind(library_id)
        .fetch_one(pool)
        .await
        .unwrap_or(0)
    }

    pub async fn get_top_genres(pool: &SqlitePool, library_id: i64) -> Vec<(String, i64)> {
        sqlx::query_as::<_, (String, i64)>(
            r#"SELECT lc.genre AS name, COUNT(*) AS cnt
               FROM library_cache lc
               INNER JOIN library_tracks lt ON lt.cache_id = lc.id
               WHERE lt.library_id = ? AND lc.genre IS NOT NULL AND lc.genre != ''
               GROUP BY lc.genre ORDER BY cnt DESC LIMIT 10"#,
        )
        .bind(library_id)
        .fetch_all(pool)
        .await
        .unwrap_or_default()
    }

    pub async fn get_top_artists(pool: &SqlitePool, library_id: i64) -> Vec<(String, i64)> {
        sqlx::query_as::<_, (String, i64)>(
            // Les deux sources : l'artiste porté par la piste, et ceux de la
            // table de liaison, souvent vide.
            r#"SELECT a.name, COUNT(*) AS cnt
               FROM (
                   SELECT lt.artist_id AS artist_id, lt.id AS track_id
                   FROM library_tracks lt
                   WHERE lt.library_id = ?1 AND lt.artist_id IS NOT NULL

                   UNION

                   SELECT lta.artist_id, lt.id
                   FROM library_track_artists lta
                   INNER JOIN library_tracks lt ON lt.id = lta.library_track_id
                   WHERE lta.library_id = ?1
               ) paires
               INNER JOIN artists a ON a.id = paires.artist_id
               GROUP BY a.id ORDER BY cnt DESC LIMIT 10"#,
        )
        .bind(library_id)
        .fetch_all(pool)
        .await
        .unwrap_or_default()
    }

    pub async fn get_top_played(
        pool: &SqlitePool,
        library_id: i64,
    ) -> Vec<(String, String, i64, Option<String>)> {
        sqlx::query_as::<_, (String, String, i64, Option<String>)>(
            r#"SELECT
                COALESCE(lt.title, '') AS title,
                COALESCE(lt.artist, '') AS artist,
                lt.play_count,
                lc.thumbnail_path
               FROM library_tracks lt
               LEFT JOIN library_cache lc ON lc.id = lt.cache_id
               WHERE lt.library_id = ? AND lt.play_count > 0
               ORDER BY lt.play_count DESC LIMIT 10"#,
        )
        .bind(library_id)
        .fetch_all(pool)
        .await
        .unwrap_or_default()
    }
}
