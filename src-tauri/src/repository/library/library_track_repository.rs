use sqlx::SqlitePool;
use uuid::Uuid;

use crate::mapper::library::track::track_detail_view::TrackDetailView;
use crate::{entity::library::library_track::{LibraryTrack, LibraryTrackCreate}, mapper::library::track::track_list_item_view::TrackListView};

pub struct LibraryTrackRepository;

impl LibraryTrackRepository {

    pub async fn insert_library_track<'e, E>(
        exec: E,
        track_data: LibraryTrackCreate
    ) -> Result<LibraryTrack, sqlx::Error>
    where
        E: sqlx::Executor<'e, Database = sqlx::Sqlite>
    {

        let id: String = Uuid::new_v4().to_string();

        let result: LibraryTrack = sqlx::query_as::<_, LibraryTrack>(
            r#"
            INSERT INTO library_tracks (
                id,
                library_id,
                file_id,
                cache_id,
                artist_id,
                library_album_id,
                title,
                title_normalized,
                track_number,
                disc_number,
                tags,
                duration,
                bitrate,
                sample_rate,
                rating,
                play_count,
                favorite,
                created_at,
                updated_at
            )
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, 0, 0, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)
            ON CONFLICT(library_id, file_id) DO UPDATE SET
                id = library_tracks.id
            RETURNING *
            "#
        )
        .bind(&id)
        .bind(&track_data.library_id)
        .bind(&track_data.file_id)
        .bind(&track_data.cache_id)
        .bind(&track_data.artist_id)
        .bind(&track_data.library_album_id)
        .bind(&track_data.title)
        .bind(&track_data.title_normalized)
        .bind(&track_data.track_number)
        .bind(&track_data.disc_number)
        .bind(&track_data.tags)
        .bind(&track_data.duration)
        .bind(&track_data.bitrate)
        .bind(&track_data.sample_rate)
        .bind(&track_data.rating)
        .fetch_one(exec)
        .await?;

        Ok(result)
    }

    pub async fn update_rating<'e, E>(
        exec: E,
        track_id: &str,
        rating: Option<f64>,
    ) -> Result<(), sqlx::Error>
    where
        E: sqlx::Executor<'e, Database = sqlx::Sqlite>
    {
        sqlx::query("UPDATE library_tracks SET rating = ?, updated_at = CURRENT_TIMESTAMP WHERE id = ?")
            .bind(rating)
            .bind(track_id)
            .execute(exec)
            .await?;
        Ok(())
    }

    pub async fn find_by_normalized_title<'e, E>(
        exec: E,
        title_normalized: &str,
        library_id: i64,
        artist_id: Option<&str>,
    ) -> Result<Option<LibraryTrack>, sqlx::Error>
    where
        E: sqlx::Executor<'e, Database = sqlx::Sqlite>
    {

        sqlx::query_as::<_, LibraryTrack>(
            r#"
            SELECT * FROM library_tracks
            WHERE title_normalized = ?
            AND library_id = ?
            AND artist_id IS ?
            LIMIT 1
            "#
        )
        .bind(title_normalized)
        .bind(library_id)
        .bind(artist_id)
        .fetch_optional(exec)
        .await
    }

    pub async fn find_by_album<'e, E>(
        exec: E,
        library_album_id: &str,
        library_id: i64,
    ) -> Result<Vec<LibraryTrack>, sqlx::Error>
    where
        E: sqlx::Executor<'e, Database = sqlx::Sqlite>
    {

        sqlx::query_as::<_, LibraryTrack>(
            r#"
            SELECT * FROM library_tracks
            WHERE library_album_id = ?
            AND library_id = ?
            ORDER BY disc_number, track_number
            "#
        )
        .bind(library_album_id)
        .bind(library_id)
        .fetch_all(exec)
        .await
    }

    pub async fn find_by_artist<'e, E>(
        exec: E,
        artist_id: &str,
        library_id: i64,
    ) -> Result<Vec<LibraryTrack>, sqlx::Error>
    where
        E: sqlx::Executor<'e, Database = sqlx::Sqlite>
    {

        sqlx::query_as::<_, LibraryTrack>(
            r#"
            SELECT DISTINCT lt.* FROM library_tracks lt
            LEFT JOIN library_track_artists lta ON lta.library_track_id = lt.id
            WHERE (lta.artist_id = ?1 OR lt.artist_id = ?1)
            AND lt.library_id = ?2
            ORDER BY lt.title
            "#
        )
        .bind(artist_id)
        .bind(library_id)
        .fetch_all(exec)
        .await
    }

    pub async fn find_by_file_id<'e, E>(
        exec: E,
        file_id: &str,
    ) -> Result<Option<LibraryTrack>, sqlx::Error>
    where
        E: sqlx::Executor<'e, Database = sqlx::Sqlite>
    {

        sqlx::query_as::<_, LibraryTrack>(
            "SELECT * FROM library_tracks WHERE file_id = ? LIMIT 1"
        )
        .bind(file_id)
        .fetch_optional(exec)
        .await
    }

    pub async fn increment_play_count<'e, E>(
        exec: E,
        track_id: &str,
    ) -> Result<(), sqlx::Error>
    where
        E: sqlx::Executor<'e, Database = sqlx::Sqlite>
    {

        sqlx::query(
            r#"
            UPDATE library_tracks
            SET play_count = play_count + 1,
                last_played_at = CURRENT_TIMESTAMP,
                updated_at = CURRENT_TIMESTAMP
            WHERE id = ?
            "#
        )
        .bind(track_id)
        .execute(exec)
        .await?;

        Ok(())
    }

    pub async fn find_all_tracks_by_library_id<'e, E>(
        exec: E,
        library_id: i64,
    ) -> Result<Vec<TrackListView>, sqlx::Error>
    where
        E: sqlx::Executor<'e, Database = sqlx::Sqlite>
    {

        let tracks: Vec<TrackListView> = sqlx::query_as::<_, TrackListView>(
            r#"
            SELECT
                -- =========================
                -- TRACK
                -- =========================
                lt.id                             AS id,
                lt.title                          AS title,
                lt.title_normalized               AS title_normalized,
                COALESCE(lt.track_number, lc.track_number) AS track_number,
                COALESCE(lt.disc_number, lc.disc_number, 1) AS disc_number,

                COALESCE(lt.duration, lc.duration)         AS duration,
                COALESCE(lt.bitrate, lc.bitrate)           AS bitrate,
                COALESCE(lt.sample_rate, lc.sample_rate)   AS sample_rate,

                lt.play_count                      AS play_count,
                lt.last_played_at                  AS last_played_at,
                lt.rating                          AS rating,
                lt.favorite                        AS favorite,
                -- Les tags ne sont pas transportés ici, et c'est délibéré.
                --
                -- Cette requête charge la bibliothèque entière d'un coup, et le
                -- résultat est ensuite gardé en mémoire et mis en cache côté
                -- interface. Y joindre le JSON des tags fait passer la charge
                -- de 6,9 à 18,3 Mo sur neuf mille pistes — mesuré — pour une
                -- vue qui n'affiche que des cartes, sans colonne de tag.
                --
                -- La colonne reste sélectionnée, à NULL : `TrackListView`
                -- l'exige, et une requête qui ne la fournirait pas échouerait à
                -- l'exécution sans que le compilateur puisse le signaler.
                --
                -- Toute vue en tableau passe par une requête ciblée — album,
                -- artiste, genre, ou la requête paginée — qui les porte.
                NULL                               AS tags,
                lt.created_at                      AS created_at,
                lt.updated_at                      AS updated_at,

                -- =========================
                -- FILE
                -- =========================
                lf.path                            AS path,
                lf.filename                        AS filename,
                lf.extension                       AS extension,
                lf.size                            AS size,
                lf.status                          AS status,
                lf.is_available                    AS is_available,
                lf.error_message                   AS error_message,

                -- =========================
                -- ARTIST / ALBUM
                -- =========================
                a.id                               AS artist_id,
                lat.id                             AS library_artist_id,
                a.name                             AS artist,
                la.id                              AS album_id,
                la.title                           AS album,

                -- =========================
                -- CACHE
                -- =========================
                lc.album_artist                    AS album_artist,
                lc.year                            AS year,
                lc.genre                           AS genre,

                lc.bits_per_sample                 AS bits_per_sample,
                lc.channels                        AS channels,
                lc.audio_format                    AS audio_format,
                lc.mime_type                       AS mime_type,
                lc.file_size                       AS file_size,
                lc.extra_tags                      AS extra_tags,
                lc.thumbnail_path                  AS thumbnail_path,
                lc.last_scanned_at                 AS last_scanned_at

            FROM library_tracks lt
            INNER JOIN library_files lf ON lf.id = lt.file_id
            LEFT JOIN library_cache lc ON lc.id = lt.cache_id
            LEFT JOIN library_albums la ON la.id = lt.library_album_id
            LEFT JOIN artists a ON a.id = lt.artist_id
            LEFT JOIN library_artists lat ON lat.artist_id = a.id AND lat.library_id = lt.library_id

            WHERE lt.library_id = ?
            ORDER BY artist, album, disc_number, track_number
            "#
        )
        .bind(library_id)
        .fetch_all(exec)
        .await?;

        Ok(tracks)
    }

    /// Tracks paginés avec filtre et tri dynamique
    pub async fn find_tracks_paginated(
        pool: &SqlitePool,
        library_id: i64,
        offset: i64,
        limit: i64,
        sort_col: &str,
        // Valeur à lier au `?` que `sort_col` peut contenir — le nom d'un tag,
        // tenu hors du texte de la requête. Voir `resolve_sort`.
        sort_bind: Option<&str>,
        sort_dir: &str,
        filter: Option<&str>,
        missing_cover: bool,
    ) -> Result<(Vec<TrackListView>, i64), sqlx::Error> {

        // Préparer le pattern LIKE une seule fois
        let like_pattern = filter.map(|f| format!("%{}%", f.to_lowercase()));

        let filter_clause = if like_pattern.is_some() {
            "AND (LOWER(lt.title) LIKE ? OR LOWER(a.name) LIKE ? OR LOWER(la.title) LIKE ?)"
        } else {
            ""
        };

        let cover_clause = if missing_cover {
            "AND (lc.thumbnail_path IS NULL OR lc.thumbnail_path = '')"
        } else {
            ""
        };

        // Count total
        let count_sql = format!(
            r#"SELECT COUNT(*)
               FROM library_tracks lt
               INNER JOIN library_files lf ON lf.id = lt.file_id
               LEFT JOIN library_cache lc ON lc.id = lt.cache_id
               LEFT JOIN library_albums la ON la.id = lt.library_album_id
               LEFT JOIN artists a ON a.id = lt.artist_id
               WHERE lt.library_id = ?
               {}
               {}"#,
            filter_clause, cover_clause
        );

        let mut count_query = sqlx::query_scalar::<_, i64>(&count_sql)
            .bind(library_id);
        if let Some(ref pat) = like_pattern {
            count_query = count_query.bind(pat).bind(pat).bind(pat);
        }
        let total = count_query.fetch_one(&*pool).await.unwrap_or(0);

        // Data query
        let data_sql = format!(
            r#"SELECT
                lt.id AS id, lt.title AS title, lt.title_normalized AS title_normalized,
                COALESCE(lt.track_number, lc.track_number) AS track_number,
                COALESCE(lt.disc_number, lc.disc_number, 1) AS disc_number,
                COALESCE(lt.duration, lc.duration) AS duration,
                COALESCE(lt.bitrate, lc.bitrate) AS bitrate,
                COALESCE(lt.sample_rate, lc.sample_rate) AS sample_rate,
                lt.play_count AS play_count, lt.last_played_at AS last_played_at,
                lt.rating AS rating, lt.favorite AS favorite,
                lt.tags AS tags,
                lt.created_at AS created_at, lt.updated_at AS updated_at,
                lf.path AS path, lf.filename AS filename, lf.extension AS extension,
                lf.size AS size, lf.status AS status, lf.is_available AS is_available,
                lf.error_message AS error_message,
                a.id AS artist_id, lat.id AS library_artist_id,
                a.name AS artist, la.id AS album_id, la.title AS album,
                lc.album_artist AS album_artist, lc.year AS year, lc.genre AS genre,
                lc.bits_per_sample AS bits_per_sample, lc.channels AS channels,
                lc.audio_format AS audio_format, lc.mime_type AS mime_type,
                lc.file_size AS file_size, lc.extra_tags AS extra_tags,
                lc.thumbnail_path AS thumbnail_path, lc.last_scanned_at AS last_scanned_at
            FROM library_tracks lt
            INNER JOIN library_files lf ON lf.id = lt.file_id
            LEFT JOIN library_cache lc ON lc.id = lt.cache_id
            LEFT JOIN library_albums la ON la.id = lt.library_album_id
            LEFT JOIN artists a ON a.id = lt.artist_id
            LEFT JOIN library_artists lat ON lat.artist_id = a.id AND lat.library_id = lt.library_id
            WHERE lt.library_id = ?
            {}
            {}
            ORDER BY {} {}
            LIMIT ? OFFSET ?"#,
            filter_clause, cover_clause, sort_col, sort_dir
        );

        // L'ordre des liaisons suit celui des `?` dans le texte : la clause
        // WHERE, puis l'expression de tri qui la suit dans l'ORDER BY, puis
        // LIMIT et OFFSET. Intervertir ces deux-là ferait trier sur « 100 ».
        let mut data_query = sqlx::query_as::<_, TrackListView>(&data_sql)
            .bind(library_id);
        if let Some(ref pat) = like_pattern {
            data_query = data_query.bind(pat).bind(pat).bind(pat);
        }
        if let Some(nom) = sort_bind {
            data_query = data_query.bind(nom);
        }
        let data_query = data_query.bind(limit).bind(offset);
        let tracks = data_query.fetch_all(&*pool).await?;

        Ok((tracks, total))
    }

    // Retourne les TrackListView d'un artiste (via artist_id direct OU library_track_artists)
    pub async fn find_tracks_view_by_artist_id<'e, E>(
        exec: E,
        library_id: i64,
        artist_id: &str,
    ) -> Result<Vec<TrackListView>, sqlx::Error>
    where
        E: sqlx::Executor<'e, Database = sqlx::Sqlite>
    {
        // Two indexed lookups via UNION: direct artist_id + library_track_artists
        sqlx::query_as::<_, TrackListView>(
            r#"
            SELECT
                lt.id AS id, lt.title AS title, lt.title_normalized AS title_normalized,
                COALESCE(lt.track_number, lc.track_number) AS track_number,
                COALESCE(lt.disc_number, lc.disc_number, 1) AS disc_number,
                COALESCE(lt.duration, lc.duration) AS duration,
                COALESCE(lt.bitrate, lc.bitrate) AS bitrate,
                COALESCE(lt.sample_rate, lc.sample_rate) AS sample_rate,
                lt.play_count AS play_count, lt.last_played_at AS last_played_at,
                lt.rating AS rating, lt.favorite AS favorite,
                lt.tags AS tags,
                lt.created_at AS created_at, lt.updated_at AS updated_at,
                lf.path AS path, lf.filename AS filename, lf.extension AS extension,
                lf.size AS size, lf.status AS status, lf.is_available AS is_available,
                lf.error_message AS error_message,
                a.id AS artist_id, lat.id AS library_artist_id, a.name AS artist,
                la.id AS album_id, la.title AS album,
                lc.album_artist AS album_artist, lc.year AS year, lc.genre AS genre,
                lc.bits_per_sample AS bits_per_sample, lc.channels AS channels,
                lc.audio_format AS audio_format, lc.mime_type AS mime_type,
                lc.file_size AS file_size, lc.extra_tags AS extra_tags,
                lc.thumbnail_path AS thumbnail_path, lc.last_scanned_at AS last_scanned_at
            FROM library_tracks lt
            INNER JOIN library_files lf ON lf.id = lt.file_id
            LEFT JOIN library_cache lc ON lc.id = lt.cache_id
            LEFT JOIN library_albums la ON la.id = lt.library_album_id
            LEFT JOIN artists a ON a.id = lt.artist_id
            LEFT JOIN library_artists lat ON lat.artist_id = a.id AND lat.library_id = lt.library_id
            WHERE lt.library_id = ?1
              AND lt.id IN (
                SELECT id FROM library_tracks WHERE artist_id = ?2 AND library_id = ?1
                UNION
                SELECT library_track_id FROM library_track_artists WHERE artist_id = ?2 AND library_id = ?1
              )
            ORDER BY la.title, lt.disc_number, lt.track_number
            "#
        )
        .bind(library_id)
        .bind(artist_id)
        .fetch_all(exec)
        .await
    }

    pub async fn find_tracks_view_by_artist_id_paginated<'e, E>(
        exec: E,
        library_id: i64,
        artist_id: &str,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<TrackListView>, sqlx::Error>
    where
        E: sqlx::Executor<'e, Database = sqlx::Sqlite>
    {
        sqlx::query_as::<_, TrackListView>(
            r#"
            SELECT DISTINCT
                lt.id AS id, lt.title AS title, lt.title_normalized AS title_normalized,
                COALESCE(lt.track_number, lc.track_number) AS track_number,
                COALESCE(lt.disc_number, lc.disc_number, 1) AS disc_number,
                COALESCE(lt.duration, lc.duration) AS duration,
                COALESCE(lt.bitrate, lc.bitrate) AS bitrate,
                COALESCE(lt.sample_rate, lc.sample_rate) AS sample_rate,
                lt.play_count AS play_count, lt.last_played_at AS last_played_at,
                lt.rating AS rating, lt.favorite AS favorite,
                lt.tags AS tags,
                lt.created_at AS created_at, lt.updated_at AS updated_at,
                lf.path AS path, lf.filename AS filename, lf.extension AS extension,
                lf.size AS size, lf.status AS status, lf.is_available AS is_available,
                lf.error_message AS error_message,
                a.id AS artist_id, lat.id AS library_artist_id, a.name AS artist,
                la.id AS album_id, la.title AS album,
                lc.album_artist AS album_artist, lc.year AS year, lc.genre AS genre,
                lc.bits_per_sample AS bits_per_sample, lc.channels AS channels,
                lc.audio_format AS audio_format, lc.mime_type AS mime_type,
                lc.file_size AS file_size, lc.extra_tags AS extra_tags,
                lc.thumbnail_path AS thumbnail_path, lc.last_scanned_at AS last_scanned_at
            FROM library_tracks lt
            INNER JOIN library_files lf ON lf.id = lt.file_id
            LEFT JOIN library_cache lc ON lc.id = lt.cache_id
            LEFT JOIN library_albums la ON la.id = lt.library_album_id
            LEFT JOIN artists a ON a.id = lt.artist_id
            LEFT JOIN library_artists lat ON lat.artist_id = a.id AND lat.library_id = lt.library_id
            WHERE lt.library_id = ?1
              AND (lt.artist_id = ?2 OR lt.id IN (
                SELECT lta.library_track_id FROM library_track_artists lta WHERE lta.artist_id = ?2
              ))
            ORDER BY la.title, lt.disc_number, lt.track_number
            LIMIT ?3 OFFSET ?4
            "#
        )
        .bind(library_id)
        .bind(artist_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(exec)
        .await
    }

    pub async fn count_tracks_by_artist_id<'e, E>(
        exec: E,
        library_id: i64,
        artist_id: &str,
    ) -> Result<i64, sqlx::Error>
    where
        E: sqlx::Executor<'e, Database = sqlx::Sqlite>
    {
        let row: (i64,) = sqlx::query_as(
            r#"
            SELECT COUNT(DISTINCT lt.id)
            FROM library_tracks lt
            WHERE lt.library_id = ?1
              AND (lt.artist_id = ?2 OR lt.id IN (
                SELECT lta.library_track_id FROM library_track_artists lta WHERE lta.artist_id = ?2
              ))
            "#
        )
        .bind(library_id)
        .bind(artist_id)
        .fetch_one(exec)
        .await?;
        Ok(row.0)
    }

    // Retourne les tracks d'un dossier spécifique (library_dir_id)
    pub async fn find_tracks_by_dir_id<'e, E>(
        exec: E,
        library_id: i64,
        dir_id: &str,
    ) -> Result<Vec<TrackListView>, sqlx::Error>
    where
        E: sqlx::Executor<'e, Database = sqlx::Sqlite>
    {
        sqlx::query_as::<_, TrackListView>(
            r#"
            SELECT
                lt.id AS id, lt.title AS title, lt.title_normalized AS title_normalized,
                COALESCE(lt.track_number, lc.track_number) AS track_number,
                COALESCE(lt.disc_number, lc.disc_number, 1) AS disc_number,
                COALESCE(lt.duration, lc.duration) AS duration,
                COALESCE(lt.bitrate, lc.bitrate) AS bitrate,
                COALESCE(lt.sample_rate, lc.sample_rate) AS sample_rate,
                lt.play_count AS play_count, lt.last_played_at AS last_played_at,
                lt.rating AS rating, lt.favorite AS favorite,
                lt.tags AS tags,
                lt.created_at AS created_at, lt.updated_at AS updated_at,
                lf.path AS path, lf.filename AS filename, lf.extension AS extension,
                lf.size AS size, lf.status AS status, lf.is_available AS is_available,
                lf.error_message AS error_message,
                a.id AS artist_id, lat.id AS library_artist_id, a.name AS artist,
                la.id AS album_id, la.title AS album,
                lc.album_artist AS album_artist, lc.year AS year, lc.genre AS genre,
                lc.bits_per_sample AS bits_per_sample, lc.channels AS channels,
                lc.audio_format AS audio_format, lc.mime_type AS mime_type,
                lc.file_size AS file_size, lc.extra_tags AS extra_tags,
                lc.thumbnail_path AS thumbnail_path, lc.last_scanned_at AS last_scanned_at
            FROM library_tracks lt
            INNER JOIN library_files lf ON lf.id = lt.file_id
            LEFT JOIN library_cache lc ON lc.id = lt.cache_id
            LEFT JOIN library_albums la ON la.id = lt.library_album_id
            LEFT JOIN artists a ON a.id = lt.artist_id
            LEFT JOIN library_artists lat ON lat.artist_id = a.id AND lat.library_id = lt.library_id
            WHERE lt.library_id = ? AND lf.library_dir_id = ?
            ORDER BY lf.path, lt.disc_number, lt.track_number
            "#
        )
        .bind(library_id)
        .bind(dir_id)
        .fetch_all(exec)
        .await
    }

    // Retourne un TrackListView complet pour un file_id donné
    // Utilisé pour récupérer la vue d'un track déjà importé sans re-analyser le fichier
    pub async fn find_track_view_by_file_id<'e, E>(
        exec: E,
        file_id: &str,
    ) -> Result<Option<TrackListView>, sqlx::Error>
    where
        E: sqlx::Executor<'e, Database = sqlx::Sqlite>
    {
        sqlx::query_as::<_, TrackListView>(
            r#"
            SELECT
                lt.id AS id, lt.title AS title, lt.title_normalized AS title_normalized,
                COALESCE(lt.track_number, lc.track_number) AS track_number,
                COALESCE(lt.disc_number, lc.disc_number, 1) AS disc_number,
                COALESCE(lt.duration, lc.duration) AS duration,
                COALESCE(lt.bitrate, lc.bitrate) AS bitrate,
                COALESCE(lt.sample_rate, lc.sample_rate) AS sample_rate,
                lt.play_count AS play_count, lt.last_played_at AS last_played_at,
                lt.rating AS rating, lt.favorite AS favorite,
                lt.tags AS tags,
                lt.created_at AS created_at, lt.updated_at AS updated_at,
                lf.path AS path, lf.filename AS filename, lf.extension AS extension,
                lf.size AS size, lf.status AS status, lf.is_available AS is_available,
                lf.error_message AS error_message,
                a.id AS artist_id, lat.id AS library_artist_id, a.name AS artist,
                la.id AS album_id, la.title AS album,
                lc.album_artist AS album_artist, lc.year AS year, lc.genre AS genre,
                lc.bits_per_sample AS bits_per_sample, lc.channels AS channels,
                lc.audio_format AS audio_format, lc.mime_type AS mime_type,
                lc.file_size AS file_size, lc.extra_tags AS extra_tags,
                lc.thumbnail_path AS thumbnail_path, lc.last_scanned_at AS last_scanned_at
            FROM library_tracks lt
            INNER JOIN library_files lf ON lf.id = lt.file_id
            LEFT JOIN library_cache lc ON lc.id = lt.cache_id
            LEFT JOIN library_albums la ON la.id = lt.library_album_id
            LEFT JOIN artists a ON a.id = lt.artist_id
            LEFT JOIN library_artists lat ON lat.artist_id = a.id AND lat.library_id = lt.library_id
            WHERE lt.file_id = ?
            LIMIT 1
            "#
        )
        .bind(file_id)
        .fetch_optional(exec)
        .await
    }

    pub async fn find_all_tracks_album_by_library_id<'e, E>(
        exec: E,
        library_id: i64,
        library_album_id: String
    ) -> Result<Vec<TrackListView>, sqlx::Error>
    where
        E: sqlx::Executor<'e, Database = sqlx::Sqlite>
    {

        let tracks: Vec<TrackListView> = sqlx::query_as::<_, TrackListView>(
            r#"
            SELECT
                -- =========================
                -- TRACK
                -- =========================
                lt.id                             AS id,
                lt.title                          AS title,
                lt.title_normalized               AS title_normalized,
                COALESCE(lt.track_number, lc.track_number) AS track_number,
                COALESCE(lt.disc_number, lc.disc_number, 1) AS disc_number,

                COALESCE(lt.duration, lc.duration)         AS duration,
                COALESCE(lt.bitrate, lc.bitrate)           AS bitrate,
                COALESCE(lt.sample_rate, lc.sample_rate)   AS sample_rate,

                lt.play_count                      AS play_count,
                lt.last_played_at                  AS last_played_at,
                lt.rating                          AS rating,
                lt.favorite                        AS favorite,
                lt.tags                            AS tags,
                lt.created_at                      AS created_at,
                lt.updated_at                      AS updated_at,

                -- =========================
                -- FILE
                -- =========================
                lf.path                            AS path,
                lf.filename                        AS filename,
                lf.extension                       AS extension,
                lf.size                            AS size,
                lf.status                          AS status,
                lf.is_available                    AS is_available,
                lf.error_message                   AS error_message,

                -- =========================
                -- ARTIST / ALBUM
                -- =========================
                a.id                               AS artist_id,
                lat.id                             AS library_artist_id,
                a.name                             AS artist,
                la.id                              AS album_id,
                la.title                           AS album,

                -- =========================
                -- CACHE
                -- =========================
                lc.album_artist                    AS album_artist,
                lc.year                            AS year,
                lc.genre                           AS genre,

                lc.bits_per_sample                 AS bits_per_sample,
                lc.channels                        AS channels,
                lc.audio_format                    AS audio_format,
                lc.mime_type                       AS mime_type,
                lc.file_size                       AS file_size,
                lc.extra_tags                      AS extra_tags,
                lc.thumbnail_path                  AS thumbnail_path,
                lc.last_scanned_at                 AS last_scanned_at

            FROM library_tracks lt
            INNER JOIN library_files lf ON lf.id = lt.file_id
            LEFT JOIN library_cache lc ON lc.id = lt.cache_id
            LEFT JOIN library_albums la ON la.id = lt.library_album_id
            LEFT JOIN artists a ON a.id = lt.artist_id
            LEFT JOIN library_artists lat ON lat.artist_id = a.id AND lat.library_id = lt.library_id

            WHERE lt.library_id = ? AND lt.library_album_id = ?
            ORDER BY artist, album, disc_number, track_number
            "#
        )
        .bind(library_id)
        .bind(library_album_id)
        .fetch_all(exec)
        .await?;

        Ok(tracks)
    }

    pub async fn find_track_by_id<'e, E>(
        exec: E,
        library_track_id: String, // ⚠️ id = String chez toi
    ) -> Result<TrackDetailView, sqlx::Error>
    where
        E: sqlx::Executor<'e, Database = sqlx::Sqlite>
    {

        let track: TrackDetailView = sqlx::query_as::<_, TrackDetailView>(
            r#"
            SELECT
                -- =========================
                -- IDENTITÉ
                -- =========================
                lt.id                               AS id,
                lt.library_id                       AS library_id,

                -- =========================
                -- TRACK
                -- =========================
                lt.title                            AS title,
                lt.title_normalized                 AS title_normalized,

                COALESCE(lt.track_number, lc.track_number)
                                                    AS track_number,

                COALESCE(lt.disc_number, lc.disc_number, 1)
                                                    AS disc_number,

                CAST(COALESCE(lt.duration, lc.duration) AS REAL)
                                                    AS duration,

                COALESCE(lt.bitrate, lc.bitrate)     AS bitrate,

                COALESCE(lt.sample_rate, lc.sample_rate)
                                                    AS sample_rate,

                lt.play_count                       AS play_count,
                lt.last_played_at                   AS last_played_at,
                lt.rating                           AS rating,
                lt.favorite                         AS favorite,
                lt.created_at                       AS created_at,
                lt.updated_at                       AS updated_at,

                -- =========================
                -- FILE
                -- =========================
                lf.path                             AS path,
                lf.filename                         AS filename,
                lf.extension                        AS extension,
                lf.size                             AS size,
                lf.status                           AS status,
                lf.is_available                     AS is_available,
                lf.error_message                    AS error_message,

                -- =========================
                -- ARTIST / ALBUM
                -- =========================
                a.name                              AS artist,
                lart.id                             AS library_artist_id,
                la.title                            AS album,
                la.id                               AS album_id,
                la.cover_url                        AS cover_url,

                -- =========================
                -- CACHE
                -- =========================
                lc.album_artist                     AS album_artist,
                lc.year                             AS year,
                lc.genre                            AS genre,

                lc.bits_per_sample                  AS bits_per_sample,
                lc.channels                         AS channels,
                lc.audio_format                     AS audio_format,
                lc.mime_type                        AS mime_type,
                lc.file_size                        AS file_size,
                lc.extra_tags                       AS extra_tags,
                lc.thumbnail_path                   AS thumbnail_path,
                lc.last_scanned_at                  AS last_scanned_at

            FROM library_tracks lt
            INNER JOIN library_files lf
                ON lf.id = lt.file_id

            LEFT JOIN library_cache lc
                ON lc.id = lt.cache_id

            LEFT JOIN library_albums la
                ON la.id = lt.library_album_id

            LEFT JOIN artists a
                ON a.id = lt.artist_id

            LEFT JOIN library_artists lart
                ON lart.artist_id = lt.artist_id
                AND lart.library_id = lt.library_id

            WHERE lt.id = ?
            LIMIT 1
            "#
        )
        .bind(library_track_id)
        .fetch_one(exec)
        .await?;

        Ok(track)
    }

    pub async fn search<'e, E>(
        exec: E,
        search_term: &str,
        limit: i64,
    ) -> Result<Vec<crate::entity::search::search_result::SearchResult>, sqlx::Error>
    where
        E: sqlx::Executor<'e, Database = sqlx::Sqlite>
    {
        sqlx::query_as::<_, crate::entity::search::search_result::SearchResult>(
            r#"
            SELECT
                lt.id AS id,
                'track' AS result_type,
                lt.title AS title,
                lc.artist AS subtitle,
                lc.thumbnail_path AS thumbnail_path,
                lf.path AS path,
                lt.library_id AS library_id
            FROM library_tracks lt
            LEFT JOIN library_cache lc ON lt.cache_id = lc.id
            LEFT JOIN library_files lf ON lt.file_id = lf.id
            WHERE LOWER(lt.title) LIKE ?1
               OR LOWER(COALESCE(lc.artist, '')) LIKE ?1
            LIMIT ?2
            "#
        )
        .bind(search_term)
        .bind(limit)
        .fetch_all(exec)
        .await
    }

    pub async fn find_tracks_by_genre<'e, E>(
        exec: E,
        library_id: i64,
        genre: &str,
    ) -> Result<Vec<TrackListView>, sqlx::Error>
    where
        E: sqlx::Executor<'e, Database = sqlx::Sqlite>
    {
        let tracks = sqlx::query_as::<_, TrackListView>(
            r#"
            SELECT
                lt.id AS id, lt.title AS title, lt.title_normalized AS title_normalized,
                COALESCE(lt.track_number, lc.track_number) AS track_number,
                COALESCE(lt.disc_number, lc.disc_number, 1) AS disc_number,
                COALESCE(lt.duration, lc.duration) AS duration,
                COALESCE(lt.bitrate, lc.bitrate) AS bitrate,
                COALESCE(lt.sample_rate, lc.sample_rate) AS sample_rate,
                lt.play_count, lt.last_played_at, lt.rating, lt.favorite,
                lt.tags,
                lt.created_at, lt.updated_at,
                lf.path, lf.filename, lf.extension, lf.size,
                lf.status, lf.is_available, lf.error_message,
                a.id AS artist_id, lat.id AS library_artist_id, a.name AS artist,
                la.id AS album_id, la.title AS album,
                lc.album_artist, lc.year, lc.genre,
                lc.bits_per_sample, lc.channels, lc.audio_format, lc.mime_type,
                lc.file_size, lc.extra_tags, lc.thumbnail_path, lc.last_scanned_at
            FROM library_tracks lt
            INNER JOIN library_files lf ON lf.id = lt.file_id
            LEFT JOIN library_cache lc ON lc.id = lt.cache_id
            LEFT JOIN library_albums la ON la.id = lt.library_album_id
            LEFT JOIN artists a ON a.id = lt.artist_id
            LEFT JOIN library_artists lat ON lat.artist_id = a.id AND lat.library_id = lt.library_id
            WHERE lt.library_id = ? AND LOWER(lc.genre) = LOWER(?)
            ORDER BY la.title, disc_number, track_number
            "#
        )
        .bind(library_id)
        .bind(genre)
        .fetch_all(exec)
        .await?;

        Ok(tracks)
    }
}
