use sqlx::Result;
use crate::entity::audio_lyrics::lyrics::{Lyrics, LyricsSource, LyricsUpsert};

pub struct LyricsRepository;

impl LyricsRepository {

    /// Récupère les paroles cachées d'un track. None si jamais fetché.
    pub async fn find_by_track_id<'e, E>(
        exec: E,
        track_id: &str,
    ) -> Result<Option<Lyrics>>
    where
        E: sqlx::Executor<'e, Database = sqlx::Sqlite>
    {
        let row = sqlx::query_as::<_, Lyrics>(
            r#"
            SELECT track_id, plain, synced, source, fetched_at, lrclib_id
            FROM lyrics
            WHERE track_id = ?
            "#
        )
        .bind(track_id)
        .fetch_optional(exec)
        .await?;

        Ok(row)
    }

    /// Insère ou met à jour les paroles d'un track.
    /// fetched_at est mis à now() automatiquement.
    pub async fn upsert<'e, E>(
        exec: E,
        data: LyricsUpsert,
    ) -> Result<Lyrics>
    where
        E: sqlx::Executor<'e, Database = sqlx::Sqlite>
    {
        let now = chrono::Utc::now().timestamp();

        let row = sqlx::query_as::<_, Lyrics>(
            r#"
            INSERT INTO lyrics (track_id, plain, synced, source, fetched_at, lrclib_id)
            VALUES (?, ?, ?, ?, ?, ?)
            ON CONFLICT(track_id) DO UPDATE SET
                plain      = excluded.plain,
                synced     = excluded.synced,
                source     = excluded.source,
                fetched_at = excluded.fetched_at,
                lrclib_id  = excluded.lrclib_id
            RETURNING track_id, plain, synced, source, fetched_at, lrclib_id
            "#
        )
        .bind(&data.track_id)
        .bind(&data.plain)
        .bind(&data.synced)
        .bind(&data.source)
        .bind(now)
        .bind(data.lrclib_id)
        .fetch_one(exec)
        .await?;

        Ok(row)
    }

    /// Marque qu'on a cherché mais rien trouvé. Évite de re-spammer LRCLIB.
    pub async fn mark_not_found<'e, E>(
        exec: E,
        track_id: &str,
    ) -> Result<()>
    where
        E: sqlx::Executor<'e, Database = sqlx::Sqlite>
    {
        let now = chrono::Utc::now().timestamp();

        sqlx::query(
            r#"
            INSERT INTO lyrics (track_id, plain, synced, source, fetched_at, lrclib_id)
            VALUES (?, NULL, NULL, ?, ?, NULL)
            ON CONFLICT(track_id) DO UPDATE SET
                source     = excluded.source,
                fetched_at = excluded.fetched_at
            "#
        )
        .bind(track_id)
        .bind(LyricsSource::None)
        .bind(now)
        .execute(exec)
        .await?;

        Ok(())
    }

    /// Supprime les paroles d'un track (force re-fetch au prochain accès).
    pub async fn delete_by_track_id<'e, E>(
        exec: E,
        track_id: &str,
    ) -> Result<()>
    where
        E: sqlx::Executor<'e, Database = sqlx::Sqlite>
    {
        sqlx::query("DELETE FROM lyrics WHERE track_id = ?")
            .bind(track_id)
            .execute(exec)
            .await?;

        Ok(())
    }
}
