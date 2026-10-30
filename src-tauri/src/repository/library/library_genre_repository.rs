use sqlx::SqlitePool;

/// Genres des pistes de la bibliothèque (`?` = bibliothèque), un nom par genre sans tenir compte
/// de la casse : l'orthographe la plus fréquente, à égalité celle à majuscule (« Classical »
/// plutôt que « classical »). `LOWER` comme le filtre des morceaux, sinon une carte n'ouvrirait
/// pas ce qu'elle annonce.
pub(crate) const GENRE_NAMES: &str = "
    tagged AS (
        SELECT lc.genre AS genre, LOWER(lc.genre) AS k, lt.id AS track_id, lt.library_album_id AS album_id,
               lt.artist_id AS artist_id, lt.last_played_at AS last_played_at
        FROM library_cache lc
        INNER JOIN library_tracks lt ON lt.cache_id = lc.id
        WHERE lt.library_id = ? AND lc.genre IS NOT NULL AND lc.genre != ''
    ),
    names AS (
        SELECT k, genre AS name FROM (
            SELECT k, genre, ROW_NUMBER() OVER (PARTITION BY k ORDER BY COUNT(*) DESC, genre) AS r
            FROM tagged GROUP BY k, genre
        ) WHERE r = 1
    )";

pub struct LibraryGenreRepository;

impl LibraryGenreRepository {
    /// Récupère tous les genres d'une bibliothèque avec stats (albums, titres, dernière écoute)
    pub async fn find_all_genres(
        pool: &SqlitePool,
        library_id: i64,
    ) -> Result<Vec<(String, i64, i64, Option<String>)>, sqlx::Error> {
        sqlx::query_as::<_, (String, i64, i64, Option<String>)>(sqlx::AssertSqlSafe(format!(
            "WITH {GENRE_NAMES}
             SELECT n.name,
                    COUNT(DISTINCT la.id) AS total_albums,
                    COUNT(DISTINCT g.track_id) AS total_tracks,
                    MAX(g.last_played_at) AS last_played_at
             FROM tagged g
             JOIN names n ON n.k = g.k
             LEFT JOIN library_albums la ON la.id = g.album_id
             GROUP BY g.k
             ORDER BY total_tracks DESC"
        )))
        .bind(library_id)
        .fetch_all(pool)
        .await
    }

    /// Les 4 premières pochettes de chaque genre, en une requête (genre en minuscules, pochette).
    pub async fn find_all_covers(pool: &SqlitePool, library_id: i64) -> Vec<(String, String)> {
        sqlx::query_as::<_, (String, String)>(
            r#"
            SELECT k, cover_url FROM (
                SELECT k, cover_url, ROW_NUMBER() OVER (PARTITION BY k ORDER BY premier) AS rang
                FROM (
                    SELECT LOWER(genre) AS k, cover_url, MIN(rowid) AS premier
                    FROM library_albums
                    WHERE library_id = ? AND genre IS NOT NULL AND cover_url IS NOT NULL AND cover_url != ''
                    GROUP BY LOWER(genre), cover_url
                )
            )
            WHERE rang <= 4
            ORDER BY k, rang
            "#,
        )
        .bind(library_id)
        .fetch_all(pool)
        .await
        .unwrap_or_default()
    }

    /// Les trois artistes les plus présents de chaque genre (genre en minuscules, artiste).
    pub async fn find_top_artists(pool: &SqlitePool, library_id: i64) -> Vec<(String, String)> {
        sqlx::query_as::<_, (String, String)>(
            r#"
            SELECT k, name FROM (
                SELECT LOWER(lc.genre) AS k, a.name AS name,
                       ROW_NUMBER() OVER (PARTITION BY LOWER(lc.genre) ORDER BY COUNT(*) DESC, a.name) AS rang
                FROM library_tracks lt
                JOIN library_cache lc ON lc.id = lt.cache_id
                JOIN artists a ON a.id = lt.artist_id
                WHERE lt.library_id = ? AND lc.genre IS NOT NULL AND lc.genre != ''
                GROUP BY LOWER(lc.genre), a.id
            )
            WHERE rang <= 3
            ORDER BY k, rang
            "#,
        )
        .bind(library_id)
        .fetch_all(pool)
        .await
        .unwrap_or_default()
    }
    /// Genres assez fournis pour un mix : (genre, titres, artistes), les plus fournis d'abord.
    pub async fn find_mix_genres(pool: &SqlitePool, library_id: i64, min_tracks: i64, min_artists: i64) -> Result<Vec<(String, i64, i64)>, sqlx::Error> {
        sqlx::query_as(sqlx::AssertSqlSafe(format!(
            "WITH {GENRE_NAMES}
             SELECT n.name, COUNT(*) AS c, COUNT(DISTINCT g.artist_id) FROM tagged g
             JOIN names n ON n.k = g.k
             GROUP BY g.k HAVING c >= ? AND COUNT(DISTINCT g.artist_id) >= ? ORDER BY c DESC"
        )))
        .bind(library_id)
        .bind(min_tracks)
        .bind(min_artists)
        .fetch_all(pool)
        .await
    }

}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::repository::library::library_track_repository::LibraryTrackRepository;
    use sqlx::sqlite::SqlitePoolOptions;

    /// « Classical » et « classical » : une seule carte, qui ouvre tous les morceaux (signalement GitHub).
    #[tokio::test]
    async fn genres_differing_by_case_make_one_card() {
        let pool = SqlitePoolOptions::new().max_connections(1).connect("sqlite::memory:").await.unwrap();
        sqlx::migrate!("./SQL/").run(&pool).await.unwrap();
        sqlx::query("INSERT OR IGNORE INTO profil (id, name) VALUES (1, 'T')").execute(&pool).await.unwrap();
        sqlx::query("INSERT OR IGNORE INTO library (id, profil_id, name) VALUES (1, 1, 'L')").execute(&pool).await.unwrap();
        sqlx::query("INSERT INTO artists (id, name, name_normalized, sort_name) VALUES ('a1', 'Bach', 'bach', 'Bach')").execute(&pool).await.unwrap();
        for (n, genre) in [(1, "Classical"), (2, "Classical"), (3, "classical"), (4, "Jazz")] {
            let path = format!("S:/m/{n}.flac");
            sqlx::query("INSERT INTO library_cache (id, path, genre, duration) VALUES (?, ?, ?, 200)").bind(n).bind(&path).bind(genre).execute(&pool).await.unwrap();
            sqlx::query("INSERT INTO library_files (id, library_id, cache_id, path, filename, extension, size, status) VALUES (?, 1, ?, ?, 'x.flac', 'flac', 1, 'ok')")
                .bind(format!("f{n}")).bind(n).bind(&path).execute(&pool).await.unwrap();
            sqlx::query("INSERT INTO library_tracks (id, library_id, file_id, cache_id, artist_id, title, title_normalized, duration) VALUES (?, 1, ?, ?, 'a1', 't', 't', 200)")
                .bind(format!("t{n}")).bind(format!("f{n}")).bind(n).execute(&pool).await.unwrap();
        }

        let genres = LibraryGenreRepository::find_all_genres(&pool, 1).await.unwrap();
        let noms: Vec<(&str, i64)> = genres.iter().map(|g| (g.0.as_str(), g.2)).collect();
        // L'orthographe la plus fréquente donne le nom ; le compte réunit les deux.
        assert_eq!(noms, [("Classical", 3), ("Jazz", 1)]);
        assert_eq!(LibraryTrackRepository::find_tracks_by_genre(&pool, 1, "Classical").await.unwrap().len(), 3);

        let mix = LibraryGenreRepository::find_mix_genres(&pool, 1, 1, 1).await.unwrap();
        assert_eq!(mix.iter().map(|m| m.0.as_str()).collect::<Vec<_>>(), ["Classical", "Jazz"]);
        let top = crate::repository::library::library_stats_repository::LibraryStatsRepository::get_top_genres(&pool, 1).await;
        assert_eq!(top, [("Classical".to_string(), 3), ("Jazz".to_string(), 1)]);
        assert_eq!(crate::repository::library::library_stats_repository::LibraryStatsRepository::count_genres(&pool, 1).await, 2);
    }
}
