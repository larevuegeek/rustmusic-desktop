use crate::mapper::recent::recent_file_list_view::RecentFileListView;

pub struct RecentFileRepository;

impl RecentFileRepository {

    pub async fn get_recent_files<'e, E>(
        exec: E,
    ) -> Result<Vec<RecentFileListView>, sqlx::Error>
    where
        E: sqlx::Executor<'e, Database = sqlx::Sqlite>
    {

        let recent_files: Vec<RecentFileListView> = sqlx::query_as::<_, RecentFileListView>(
            r#"
            SELECT
            rf.id,
            rf.library_id,
            rf.path,
            rf.last_played_at,
            rf.last_position,
            rf.play_count,

            lc.id            AS cache_id,
            lc.path          AS cache_path,
            lc.title,
            lc.artist,
            lc.album,
            lc.album_artist,
            lc.year,
            lc.genre,
            lc.track_number,
            lc.disc_number,
            lc.duration,
            lc.bitrate,
            lc.bits_per_sample,
            lc.sample_rate,
            lc.channels,
            lc.audio_format,
            lc.mime_type,
            lc.file_size,
            lc.extra_tags,
            lc.thumbnail_path,
            lc.last_scanned_at,

            EXISTS(
                SELECT 1
                FROM track_liked tl
                WHERE tl.path = lc.path
            ) AS liked

            FROM recent_files rf
            LEFT JOIN library_cache lc
                ON lc.path = rf.path

            ORDER BY rf.last_played_at DESC
            -- Assez pour regrouper les dernières écoutes par album sur l'accueil.
            LIMIT 50
            "#
        )
        .fetch_all(exec)
        .await?;

        Ok(recent_files)
    }

    pub async fn insert_recent_file<'e, E>(
        exec: E,
        path: String,
        library_id: Option<i64>,
    ) -> Result<(), sqlx::Error>
    where
        E: sqlx::Executor<'e, Database = sqlx::Sqlite>
    {

        sqlx::query(
            r#"
            INSERT INTO recent_files (
                library_id,
                path,
                last_played_at,
                last_position,
                play_count
            )
            VALUES (?, ?, CURRENT_TIMESTAMP, ?, 1)
            ON CONFLICT(path) DO UPDATE SET
                library_id     = excluded.library_id,
                last_played_at = CURRENT_TIMESTAMP,
                last_position  = excluded.last_position,
                play_count     = play_count + 1
            "#
        )
        .bind(library_id)
        .bind(path)
        .bind(0)
        .execute(exec)
        .await?;

        Ok(())
    }


    pub async fn count_recent_files<'e, E>(
        exec: E,
    ) -> Result<i64, sqlx::Error>
    where
        E: sqlx::Executor<'e, Database = sqlx::Sqlite>
    {

        let count: (i64,) =
            sqlx::query_as("SELECT COUNT(*) FROM recent_files")
                .fetch_one(exec)
                .await?;

        Ok(count.0)
    }

    pub async fn clear_recent_files<'e, E>(
        exec: E,
    ) -> Result<(), sqlx::Error>
    where
        E: sqlx::Executor<'e, Database = sqlx::Sqlite>
    {

        sqlx::query("DELETE FROM recent_files")
            .execute(exec)
            .await?;

        Ok(())
    }

    pub async fn remove_recent_file<'e, E>(
        exec: E,
        path: String,
    ) -> Result<(), sqlx::Error>
    where
        E: sqlx::Executor<'e, Database = sqlx::Sqlite>
    {

        sqlx::query("DELETE FROM recent_files WHERE path = ?")
            .bind(path)
            .execute(exec)
            .await?;

        Ok(())
    }

    /// Où l'on en était dans ce morceau, pour pouvoir y revenir.
    ///
    /// L'upsert de `record_play` fixe la position à zéro — c'est juste au
    /// démarrage d'une écoute, et faux ensuite. Sans cette écriture au fil de
    /// la lecture, `last_position` ne valait jamais que zéro et aucune reprise
    /// n'était possible.
    pub async fn save_position<'e, E>(
        exec: E,
        path: &str,
        position: f64,
    ) -> Result<(), sqlx::Error>
    where
        E: sqlx::Executor<'e, Database = sqlx::Sqlite>
    {
        sqlx::query("UPDATE recent_files SET last_position = ? WHERE path = ?")
            .bind(position)
            .bind(path)
            .execute(exec)
            .await?;

        Ok(())
    }
}
