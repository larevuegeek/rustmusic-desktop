use crate::mapper::recent::recent_file_list_view::RecentFileListView;

pub struct RecentFileRepository;

impl RecentFileRepository {

    /// L'historique du profil ; `source` : celle à garder, `None` : toutes les sources actives.
    pub async fn get_recent_files<'e, E>(
        exec: E,
        profil_id: i64,
        source: Option<i64>,
    ) -> Result<Vec<RecentFileListView>, sqlx::Error>
    where
        E: sqlx::Executor<'e, Database = sqlx::Sqlite>
    {

        let sql = format!(r#"
            SELECT
            rf.id,
            rf.library_cache_id,
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
                WHERE tl.path = rf.path AND tl.profil_id = rf.profil_id
            ) AS liked

            FROM recent_files rf
            LEFT JOIN library_cache lc
                ON lc.path = rf.path
            WHERE rf.profil_id = ? AND {}
            ORDER BY rf.last_played_at DESC
            -- Assez pour regrouper les dernières écoutes par album sur l'accueil.
            LIMIT 50
            "#, if source.is_some() { "rf.source_id = ?".to_string() } else { crate::core::remote::enabled_source("rf.source_id") });
        let mut query = sqlx::query_as::<_, RecentFileListView>(sqlx::AssertSqlSafe(sql)).bind(profil_id);
        if let Some(s) = source {
            query = query.bind(s);
        }
        let recent_files = query.fetch_all(exec).await?;

        Ok(recent_files)
    }

    /// Une écoute de plus ; l'entrée du cache et la source se déduisent du chemin.
    pub async fn insert_recent_file<'e, E>(
        exec: E,
        profil_id: i64,
        path: String,
    ) -> Result<(), sqlx::Error>
    where
        E: sqlx::Executor<'e, Database = sqlx::Sqlite>
    {

        sqlx::query(
            r#"
            INSERT INTO recent_files (
                profil_id,
                library_cache_id,
                path,
                last_played_at,
                last_position,
                play_count,
                source_id
            )
            VALUES (?1, (SELECT id FROM library_cache WHERE path = ?2), ?2, CURRENT_TIMESTAMP, 0, 1, ?3)
            ON CONFLICT(profil_id, path) DO UPDATE SET
                library_cache_id = excluded.library_cache_id,
                last_played_at   = CURRENT_TIMESTAMP,
                last_position    = excluded.last_position,
                play_count       = play_count + 1
            "#
        )
        .bind(profil_id)
        .bind(&path)
        .bind(crate::core::remote::source_of(&path))
        .execute(exec)
        .await?;

        Ok(())
    }


    pub async fn count_recent_files<'e, E>(
        exec: E,
        profil_id: i64,
        source: Option<i64>,
    ) -> Result<i64, sqlx::Error>
    where
        E: sqlx::Executor<'e, Database = sqlx::Sqlite>
    {
        match source {
            Some(s) => sqlx::query_scalar("SELECT COUNT(*) FROM recent_files WHERE profil_id = ? AND source_id = ?").bind(profil_id).bind(s).fetch_one(exec).await,
            None => {
                let sql = format!("SELECT COUNT(*) FROM recent_files WHERE profil_id = ? AND {}", crate::core::remote::enabled_source("source_id"));
                sqlx::query_scalar(sqlx::AssertSqlSafe(sql)).bind(profil_id).fetch_one(exec).await
            }
        }
    }

    /// Vide l'historique du profil pour une source (`None` : pour toutes).
    pub async fn clear_recent_files<'e, E>(
        exec: E,
        profil_id: i64,
        source: Option<i64>,
    ) -> Result<(), sqlx::Error>
    where
        E: sqlx::Executor<'e, Database = sqlx::Sqlite>
    {
        match source {
            Some(s) => sqlx::query("DELETE FROM recent_files WHERE profil_id = ? AND source_id = ?").bind(profil_id).bind(s).execute(exec).await?,
            None => sqlx::query("DELETE FROM recent_files WHERE profil_id = ?").bind(profil_id).execute(exec).await?,
        };
        Ok(())
    }

    pub async fn remove_recent_file<'e, E>(
        exec: E,
        profil_id: i64,
        path: String,
    ) -> Result<(), sqlx::Error>
    where
        E: sqlx::Executor<'e, Database = sqlx::Sqlite>
    {

        sqlx::query("DELETE FROM recent_files WHERE profil_id = ? AND path = ?")
            .bind(profil_id)
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
        profil_id: i64,
        path: &str,
        position: f64,
    ) -> Result<(), sqlx::Error>
    where
        E: sqlx::Executor<'e, Database = sqlx::Sqlite>
    {
        sqlx::query("UPDATE recent_files SET last_position = ? WHERE profil_id = ? AND path = ?")
            .bind(position)
            .bind(profil_id)
            .bind(path)
            .execute(exec)
            .await?;

        Ok(())
    }
}
