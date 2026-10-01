use sqlx::SqlitePool;

pub struct LibraryGenreRepository;

impl LibraryGenreRepository {
    /// Récupère tous les genres d'une bibliothèque avec stats (albums, titres, dernière écoute)
    pub async fn find_all_genres(
        pool: &SqlitePool,
        library_id: i64,
    ) -> Result<Vec<(String, i64, i64, Option<String>)>, sqlx::Error> {
        sqlx::query_as::<_, (String, i64, i64, Option<String>)>(
            r#"
            SELECT
                lc.genre AS name,
                COUNT(DISTINCT la.id) AS total_albums,
                COUNT(DISTINCT lt.id) AS total_tracks,
                MAX(lt.last_played_at) AS last_played_at
            FROM library_cache lc
            INNER JOIN library_tracks lt ON lt.cache_id = lc.id
            LEFT JOIN library_albums la ON la.id = lt.library_album_id
            WHERE lt.library_id = ?
              AND lc.genre IS NOT NULL AND lc.genre != ''
            GROUP BY lc.genre
            ORDER BY total_tracks DESC
            "#,
        )
        .bind(library_id)
        .fetch_all(pool)
        .await
    }

    /// Les 4 premières pochettes de chaque genre, en une requête (genre, pochette).
    pub async fn find_all_covers(pool: &SqlitePool, library_id: i64) -> Vec<(String, String)> {
        sqlx::query_as::<_, (String, String)>(
            r#"
            SELECT genre, cover_url FROM (
                SELECT genre, cover_url, ROW_NUMBER() OVER (PARTITION BY genre ORDER BY premier) AS rang
                FROM (
                    SELECT genre, cover_url, MIN(rowid) AS premier
                    FROM library_albums
                    WHERE library_id = ? AND genre IS NOT NULL AND cover_url IS NOT NULL AND cover_url != ''
                    GROUP BY genre, cover_url
                )
            )
            WHERE rang <= 4
            ORDER BY genre, rang
            "#,
        )
        .bind(library_id)
        .fetch_all(pool)
        .await
        .unwrap_or_default()
    }

    /// Les trois artistes les plus présents de chaque genre (genre, artiste).
    pub async fn find_top_artists(pool: &SqlitePool, library_id: i64) -> Vec<(String, String)> {
        sqlx::query_as::<_, (String, String)>(
            r#"
            SELECT genre, name FROM (
                SELECT lc.genre AS genre, a.name AS name,
                       ROW_NUMBER() OVER (PARTITION BY lc.genre ORDER BY COUNT(*) DESC, a.name) AS rang
                FROM library_tracks lt
                JOIN library_cache lc ON lc.id = lt.cache_id
                JOIN artists a ON a.id = lt.artist_id
                WHERE lt.library_id = ? AND lc.genre IS NOT NULL AND lc.genre != ''
                GROUP BY lc.genre, a.id
            )
            WHERE rang <= 3
            ORDER BY genre, rang
            "#,
        )
        .bind(library_id)
        .fetch_all(pool)
        .await
        .unwrap_or_default()
    }
}
