use uuid::Uuid;

use crate::entity::library::library_artist::LibraryArtist;
use crate::entity::library::library_artist::LibraryArtistCreate;
use crate::mapper::library::artist::artist_detail_view::ArtistDetailView;
use crate::mapper::library::artist::artist_list_view::ArtistListView;

pub struct LibraryArtistRepository;

impl LibraryArtistRepository {

    pub async fn insert_library_artist<'e, E>(
        exec: E,
        artist_data: LibraryArtistCreate
    ) -> Result<LibraryArtist, sqlx::Error>
    where
        E: sqlx::Executor<'e, Database = sqlx::Sqlite>
    {

        let id: String = Uuid::new_v4().to_string();

        let artist: LibraryArtist = sqlx::query_as::<_, LibraryArtist>(
            r#"
            INSERT INTO library_artists (
                id,
                library_id,
                artist_id,
                created_at,
                updated_at
            )
            VALUES (?, ?, ?, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)

            ON CONFLICT(library_id, artist_id)
            DO UPDATE SET
                id = library_artists.id

            RETURNING *
            "#
        )
        .bind(&id)
        .bind(&artist_data.library_id)
        .bind(&artist_data.artist_id)
        .fetch_one(exec)
        .await?;

        Ok(artist)
    }

    pub async fn find_by_artist_id<'e, E>(
        exec: E,
        library_id: i64,
        artist_id: Option<&str>,
    ) -> Result<Option<LibraryArtist>, sqlx::Error>
    where
        E: sqlx::Executor<'e, Database = sqlx::Sqlite>
    {
        sqlx::query_as::<_, LibraryArtist>(
            r#"
            SELECT * FROM library_artists
            WHERE library_id = ?
            AND artist_id IS ?
            LIMIT 1
            "#
        )
        .bind(library_id)
        .bind(artist_id)
        .fetch_optional(exec)
        .await
    }

    pub async fn find_all_artists_by_library_id<'e, E>(
        exec: E,
        library_id: i64,
    ) -> Result<Vec<ArtistListView>, sqlx::Error>
    where
        E: sqlx::Executor<'e, Database = sqlx::Sqlite>
    {

        let artists: Vec<ArtistListView> = sqlx::query_as::<_, ArtistListView>(
            r#"
            SELECT
                a.id    AS id,
                a.name  AS name,
                COALESCE(album_stats.cnt, 0) AS total_albums,
                COALESCE(track_stats.cnt, 0) AS total_tracks,
                COALESCE(track_stats.dur, 0.0) AS total_duration,
                COALESCE(NULLIF(a.image_url, ''), cover.thumbnail_path) AS thumbnail_path

            FROM library_artists lart
            INNER JOIN artists a ON a.id = lart.artist_id

            -- Nombre d'albums (1 sous-requête pré-agrégée)
            LEFT JOIN (
                SELECT artist_id, COUNT(*) AS cnt
                FROM library_albums WHERE library_id = ?1
                GROUP BY artist_id
            ) album_stats ON album_stats.artist_id = a.id

            -- Les deux rattachements : `library_tracks.artist_id` (principal)
            -- et `library_track_artists` (tous). `UNION` et non `UNION ALL`,
            -- le principal figure souvent dans les deux.
            LEFT JOIN (
                SELECT artist_id, COUNT(*) AS cnt, COALESCE(SUM(duration), 0.0) AS dur
                FROM (
                    SELECT lt.artist_id AS artist_id, lt.id AS track_id, lt.duration AS duration
                    FROM library_tracks lt
                    WHERE lt.library_id = ?1 AND lt.artist_id IS NOT NULL

                    UNION

                    SELECT lta.artist_id, lt.id, lt.duration
                    FROM library_track_artists lta
                    INNER JOIN library_tracks lt ON lt.id = lta.library_track_id
                    WHERE lta.library_id = ?1
                )
                GROUP BY artist_id
            ) track_stats ON track_stats.artist_id = a.id

            -- Première cover trouvée
            LEFT JOIN (
                SELECT lt.artist_id, lc.thumbnail_path
                FROM library_tracks lt
                INNER JOIN library_cache lc ON lc.id = lt.cache_id
                WHERE lt.library_id = ?1 AND lc.thumbnail_path IS NOT NULL
                GROUP BY lt.artist_id
            ) cover ON cover.artist_id = a.id

            WHERE lart.library_id = ?1
              -- Écarte ceux qui n'ont plus rien ici : une cascade efface les
              -- pistes mais laisse la ligne `library_artists`.
              AND (COALESCE(album_stats.cnt, 0) > 0 OR COALESCE(track_stats.cnt, 0) > 0)
            ORDER BY a.name COLLATE NOCASE ASC
            "#
        )
        .bind(library_id)
        .fetch_all(exec)
        .await?;

        Ok(artists)
    }

    /// Artistes similaires : même genre, même library, excluant l'artiste donné
    /// Requête légère : pas de stats tracks/duration, juste nom + image + nb albums
    pub async fn find_similar_artists<'e, E>(
        exec: E,
        library_id: i64,
        artist_id: &str,
        limit: i64,
    ) -> Result<Vec<ArtistListView>, sqlx::Error>
    where
        E: sqlx::Executor<'e, Database = sqlx::Sqlite>
    {
        sqlx::query_as::<_, ArtistListView>(
            r#"
            SELECT
                a.id    AS id,
                a.name  AS name,
                COUNT(DISTINCT la_all.id) AS total_albums,
                (SELECT COUNT(DISTINCT lta_s.library_track_id) FROM library_track_artists lta_s
                 WHERE lta_s.artist_id = a.id AND lta_s.library_id = ?1
                ) AS total_tracks,
                0.0 AS total_duration,
                COALESCE(NULLIF(a.image_url, ''), (
                    SELECT lc.thumbnail_path
                    FROM library_tracks lt
                    INNER JOIN library_cache lc ON lc.id = lt.cache_id
                    WHERE lt.artist_id = a.id AND lt.library_id = ?1 AND lc.thumbnail_path IS NOT NULL
                    LIMIT 1
                )) AS thumbnail_path
            FROM library_albums la
            INNER JOIN artists a ON a.id = la.artist_id
            INNER JOIN library_artists lart ON lart.artist_id = a.id AND lart.library_id = ?1
            LEFT JOIN library_albums la_all ON la_all.artist_id = a.id AND la_all.library_id = ?1
            WHERE la.library_id = ?1
              AND a.id != ?2
              AND LOWER(la.genre) IN (
                SELECT DISTINCT LOWER(la2.genre)
                FROM library_albums la2
                WHERE la2.artist_id = ?2 AND la2.library_id = ?1 AND la2.genre IS NOT NULL
              )
            GROUP BY a.id
            ORDER BY RANDOM()
            LIMIT ?3
            "#
        )
        .bind(library_id)
        .bind(artist_id)
        .bind(limit)
        .fetch_all(exec)
        .await
    }

    pub async fn find_artist_by_id<'e, E>(
        exec: E,
        library_artist_id: String,
    ) -> Result<ArtistDetailView, sqlx::Error>
    where
        E: sqlx::Executor<'e, Database = sqlx::Sqlite>
    {

        let artist: ArtistDetailView = sqlx::query_as::<_, ArtistDetailView>(
            r#"
            SELECT
                a.id    AS id,
                a.name  AS name,

                (SELECT COUNT(*) FROM library_albums la2
                 WHERE la2.artist_id = a.id
                ) AS total_albums,

                (SELECT COUNT(DISTINCT lt2.id) FROM library_tracks lt2
                 LEFT JOIN library_track_artists lta2 ON lta2.library_track_id = lt2.id AND lta2.artist_id = a.id
                 WHERE lt2.artist_id = a.id OR lta2.artist_id IS NOT NULL
                ) AS total_tracks,

                (SELECT COALESCE(SUM(lt3.duration), 0.0) FROM library_tracks lt3
                 LEFT JOIN library_track_artists lta3 ON lta3.library_track_id = lt3.id AND lta3.artist_id = a.id
                 WHERE lt3.artist_id = a.id OR lta3.artist_id IS NOT NULL
                ) AS total_duration,

                COALESCE(NULLIF(a.image_url, ''), (SELECT lc4.thumbnail_path FROM library_tracks lt4
                 INNER JOIN library_cache lc4 ON lc4.id = lt4.cache_id
                 LEFT JOIN library_track_artists lta4 ON lta4.library_track_id = lt4.id AND lta4.artist_id = a.id
                 WHERE (lt4.artist_id = a.id OR lta4.artist_id IS NOT NULL)
                 AND lc4.thumbnail_path IS NOT NULL
                 LIMIT 1
                )) AS thumbnail_path

            FROM library_artists lart
            INNER JOIN artists a ON a.id = lart.artist_id
            WHERE lart.id = ?1 OR a.id = ?1
            LIMIT 1
            "#
        )
        .bind(library_artist_id)
        .fetch_one(exec)
        .await?;

        Ok(artist)
    }

}
