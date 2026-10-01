use sqlx::SqlitePool;

use crate::entity::queue::queue_track::QueueTrack;


pub struct QueueTrackRepository;

impl QueueTrackRepository {

    pub async fn get_queue_tracks_by_profil_id<'e, E>(
            exec: E,
            profil_id: i64
        ) -> Result<Vec<QueueTrack>, sqlx::Error>
    where
        E: sqlx::Executor<'e, Database = sqlx::Sqlite>
    {

            let tracks: Vec<QueueTrack> = sqlx::query_as::<_, QueueTrack>(
            // Une file enregistrée sans titre (« Inconnu ») se complète depuis la
            // bibliothèque, qui connaît le morceau par son chemin.
            "SELECT
                q.queue_id,
                q.profil_id,
                q.position,
                q.path,
                COALESCE(NULLIF(NULLIF(q.title, ''), 'Inconnu'), c.title, q.title) AS title,
                COALESCE(q.artist, c.artist) AS artist,
                COALESCE(q.duration, c.duration) AS duration,
                COALESCE(q.cover, c.thumbnail_path) AS cover
             FROM queue_tracks q
             LEFT JOIN library_cache c ON c.path = q.path
             WHERE q.profil_id = ?
             ORDER BY q.position ASC"
        )
        .bind(profil_id)
        .fetch_all(exec)
        .await?;

        Ok(tracks)
    }

    pub async fn add_track<'e, E>(
        exec: E,
        profil_id: i64,
        track: QueueTrack
    ) -> Result<(), sqlx::Error>
    where
        E: sqlx::Executor<'e, Database = sqlx::Sqlite>
    {

        sqlx::query(
            r#"
            INSERT INTO queue_tracks
            (queue_id, profil_id, position, path, title, artist, duration, cover)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?)
            "#
        )
        .bind(&track.queue_id)
        .bind(profil_id)
        .bind(track.position)
        .bind(&track.path)
        .bind(&track.title)
        .bind(&track.artist)
        .bind(track.duration)
        .bind(&track.cover)
        .execute(exec)
        .await?;

        Ok(())
    }

    pub async fn remove_track<'e, E>(
        exec: E,
        profil_id: i64,
        queue_id: &str
    ) -> Result<(), sqlx::Error>
    where
        E: sqlx::Executor<'e, Database = sqlx::Sqlite>
    {

        sqlx::query("DELETE FROM queue_tracks WHERE queue_id = ? AND profil_id = ?")
            .bind(queue_id)
            .bind(profil_id)
            .execute(exec)
            .await?;

        Ok(())
    }

    pub async fn replace_all(pool: &SqlitePool, profil_id: i64, tracks: Vec<QueueTrack>) -> Result<(), sqlx::Error>
    {
        // On ouvre la transaction !
        let mut tx = pool.begin().await?;

        // 1. On nettoie tout pour ce profil
        sqlx::query("DELETE FROM queue_tracks WHERE profil_id = ?")
            .bind(profil_id)
            .execute(&mut *tx)
            .await?;

        // 2. On insère la nouvelle liste
        for track in tracks {
            sqlx::query(
                r#"
                INSERT INTO queue_tracks
                (queue_id, profil_id, position, path, title, artist, duration, cover)
                VALUES (?, ?, ?, ?, ?, ?, ?, ?)
                "#
            )
            .bind(&track.queue_id)
            .bind(profil_id)
            .bind(track.position)
            .bind(&track.path)
            .bind(&track.title)
            .bind(&track.artist)
            .bind(track.duration)
            .bind(&track.cover)
            .execute(&mut *tx)
            .await?;
        }

        // 3. On valide la transaction (si ça a planté avant, rien n'est appliqué en base)
        tx.commit().await?;

        Ok(())
    }

}
