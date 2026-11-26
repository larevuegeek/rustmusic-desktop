use uuid::Uuid;

use crate::entity::library::library_artist::LibraryArtist;
use crate::entity::library::library_artist::LibraryArtistCreate;
use crate::query::artist::artist_detail_row::ArtistDetailRow;
use crate::query::artist::artist_list_row::ArtistListRow;

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

    pub async fn find_all_artists_by_library_id<'e, E>(
        exec: E,
        library_id: i64,
    ) -> Result<Vec<ArtistListRow>, sqlx::Error>
    where
        E: sqlx::Executor<'e, Database = sqlx::Sqlite>
    {

        let artists: Vec<ArtistListRow> = sqlx::query_as::<_, ArtistListRow>(
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

    /// Artistes similaires : même genre, même library, excluant l'artiste donné.
    /// Les artistes sont tirés au sort d'abord, leurs comptes ne sont faits que pour eux :
    /// sur 21 000 albums, compter pour tout le genre avant le tirage prenait des secondes.
    pub async fn find_similar_artists<'e, E>(
        exec: E,
        library_id: i64,
        artist_id: &str,
        limit: i64,
    ) -> Result<Vec<ArtistListRow>, sqlx::Error>
    where
        E: sqlx::Executor<'e, Database = sqlx::Sqlite>
    {
        sqlx::query_as::<_, ArtistListRow>(
            r#"
            WITH genres AS (
                SELECT DISTINCT LOWER(genre) AS genre
                FROM library_albums
                WHERE artist_id = ?2 AND library_id = ?1 AND genre IS NOT NULL
            ),
            picked AS (
                SELECT la.artist_id AS id
                FROM library_albums la
                WHERE la.library_id = ?1
                  AND la.artist_id != ?2
                  AND LOWER(la.genre) IN (SELECT genre FROM genres)
                  AND EXISTS (SELECT 1 FROM library_artists lart WHERE lart.artist_id = la.artist_id AND lart.library_id = ?1)
                GROUP BY la.artist_id
                ORDER BY RANDOM()
                LIMIT ?3
            )
            SELECT
                a.id    AS id,
                a.name  AS name,
                (SELECT COUNT(*) FROM library_albums la_all
                 WHERE la_all.artist_id = a.id AND la_all.library_id = ?1
                ) AS total_albums,
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
            FROM picked p
            INNER JOIN artists a ON a.id = p.id
            ORDER BY RANDOM()
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
    ) -> Result<ArtistDetailRow, sqlx::Error>
    where
        E: sqlx::Executor<'e, Database = sqlx::Sqlite>
    {

        let artist: ArtistDetailRow = sqlx::query_as::<_, ArtistDetailRow>(
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

    /// Albums de chaque artiste de la bibliothèque, recomptés en un seul passage.
    pub async fn recount_albums<'e, E>(exec: E, library_id: i64) -> Result<(), sqlx::Error>
    where
        E: sqlx::Executor<'e, Database = sqlx::Sqlite>,
    {
        sqlx::query(
            "UPDATE library_artists SET total_albums = COALESCE((
                SELECT c.n FROM (
                    SELECT library_id, artist_id, COUNT(*) AS n
                    FROM library_albums GROUP BY library_id, artist_id
                ) c
                WHERE c.library_id = library_artists.library_id
                  AND c.artist_id = library_artists.artist_id), 0)
             WHERE library_id = ?",
        )
        .bind(library_id)
        .execute(exec)
        .await
        .map(|_| ())
    }

    /// Pistes de chaque artiste de la bibliothèque, recomptées en un seul passage.
    pub async fn recount_tracks<'e, E>(exec: E, library_id: i64) -> Result<(), sqlx::Error>
    where
        E: sqlx::Executor<'e, Database = sqlx::Sqlite>,
    {
        sqlx::query(
            "UPDATE library_artists SET total_tracks = COALESCE((
                SELECT c.n FROM (
                    SELECT library_id, artist_id, COUNT(*) AS n
                    FROM library_tracks WHERE artist_id IS NOT NULL
                    GROUP BY library_id, artist_id
                ) c
                WHERE c.library_id = library_artists.library_id
                  AND c.artist_id = library_artists.artist_id), 0)
             WHERE library_id = ?",
        )
        .bind(library_id)
        .execute(exec)
        .await
        .map(|_| ())
    }

}
