use crate::entity::library::library_cache::{
    LibraryCache,
    LibraryCacheCreate
};

pub struct LibraryCacheRepository;

impl LibraryCacheRepository {

    /// Insert ou update (upsert) un cache audio
    pub async fn upsert_library_cache<'e, E>(
        exec: E,
        data: LibraryCacheCreate,
    ) -> Result<LibraryCache, sqlx::Error>
    where
        E: sqlx::Executor<'e, Database = sqlx::Sqlite>
    {

        let result: LibraryCache = sqlx::query_as::<_, LibraryCache>(
            r#"
            INSERT INTO library_cache (
                path,
                title,
                artist,
                album,
                album_artist,
                year,
                genre,
                track_number,
                disc_number,
                duration,
                bitrate,
                bits_per_sample,
                sample_rate,
                channels,
                audio_format,
                mime_type,
                file_size,
                extra_tags,
                thumbnail_path,
                last_scanned_at
            )
            VALUES (
                ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, CURRENT_TIMESTAMP
            )
            ON CONFLICT(path) DO UPDATE SET
                title           = excluded.title,
                artist          = excluded.artist,
                album           = excluded.album,
                album_artist    = excluded.album_artist,
                year            = excluded.year,
                genre           = excluded.genre,
                track_number    = excluded.track_number,
                disc_number     = excluded.disc_number,
                duration        = excluded.duration,
                bitrate         = excluded.bitrate,
                bits_per_sample = excluded.bits_per_sample,
                sample_rate     = excluded.sample_rate,
                channels        = excluded.channels,
                audio_format    = excluded.audio_format,
                mime_type       = excluded.mime_type,
                file_size       = excluded.file_size,
                extra_tags      = excluded.extra_tags,
                thumbnail_path  = excluded.thumbnail_path,
                last_scanned_at = CURRENT_TIMESTAMP
            RETURNING *
            "#
        )
        .bind(&data.path)
        .bind(&data.title)
        .bind(&data.artist)
        .bind(&data.album)
        .bind(&data.album_artist)
        .bind(data.year)
        .bind(&data.genre)
        .bind(data.track_number)
        .bind(data.disc_number)
        .bind(data.duration)
        .bind(data.bitrate)
        .bind(data.bits_per_sample)
        .bind(data.sample_rate)
        .bind(data.channels)
        .bind(&data.audio_format)
        .bind(&data.mime_type)
        .bind(data.file_size)
        .bind(&data.extra_tags)
        .bind(&data.thumbnail_path)
        .fetch_one(exec)
        .await?;

        Ok(result)
    }

    pub async fn update_library_cache_thumbnail_path<'e, E>(
            exec: E,
            new_path_str: &str,
            old_path_str: &str,
        ) -> Result<(), sqlx::Error>
        where
            E: sqlx::Executor<'e, Database = sqlx::Sqlite>
        {

        sqlx::query("UPDATE library_cache SET thumbnail_path = ? WHERE thumbnail_path = ?")
            .bind(new_path_str)
            .bind(old_path_str)
            .execute(exec)
            .await?;

            Ok(())
    }

    /// Trouver un cache par path
    pub async fn find_by_path<'e, E>(
        exec: E,
        path: &str,
    ) -> Result<Option<LibraryCache>, sqlx::Error>
    where
        E: sqlx::Executor<'e, Database = sqlx::Sqlite>
    {

        sqlx::query_as::<_, LibraryCache>(
            "SELECT * FROM library_cache WHERE path = ? LIMIT 1"
        )
        .bind(path)
        .fetch_optional(exec)
        .await
    }

    pub async fn get_library_cache_id_by_path<'e, E>(
        exec: E,
        path: &str,
    ) -> Result<Option<i64>, sqlx::Error>
    where
        E: sqlx::Executor<'e, Database = sqlx::Sqlite>
    {

        sqlx::query_scalar::<_, i64>(
            "SELECT id FROM library_cache WHERE path = ?1 LIMIT 1"
        )
        .bind(path)
        .fetch_optional(exec)
        .await
    }

}
