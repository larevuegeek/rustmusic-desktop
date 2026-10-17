use sqlx::SqlitePool;


use crate::mapper::queue::queue_state_view::QueueStateView;
use crate::{entity::queue::queue_state::QueueState, repository::queue::queue_track_repository::QueueTrackRepository};
use crate::entity::queue::queue_track::QueueTrack;

pub struct QueueStateRepository;

impl QueueStateRepository {

    pub async fn get_queue(
        pool: &SqlitePool,
        profil_id: i64
    ) -> Result<QueueStateView, sqlx::Error>
    {

        let queue_state: QueueState = Self::get_queue_state_by_profile_id(pool, profil_id).await?;

        // get_queue_tracks_profil_id
        let queue_tracks: Vec<QueueTrack> = QueueTrackRepository::get_queue_tracks_by_profile_id(pool, profil_id).await?;

        Ok(QueueStateView{
            profil_id: queue_state.profil_id,
            current_index: queue_state.current_index,
            is_shuffled: queue_state.is_shuffled,
            repeat_mode: queue_state.repeat_mode,
            tracks: queue_tracks,
        })
    }

    pub async fn get_queue_state_by_profile_id<'e, E>(
            exec: E,
            profil_id: i64
        ) -> Result<QueueState, sqlx::Error>
    where
        E: sqlx::Executor<'e, Database = sqlx::Sqlite>
    {

            let queue_state: QueueState = sqlx::query_as::<_, QueueState>(
                r#"
                SELECT
                qs.profil_id,
                qs.current_index,
                qs.is_shuffled,
                qs.repeat_mode
                FROM queue_state qs
                WHERE qs.profil_id = ?
                LIMIT 1
                "#
            )
            .bind(profil_id)
            .fetch_one(exec)
            .await?;

            Ok(queue_state)
    }

    pub async fn clear_queue(pool: &SqlitePool, profil_id: i64) -> Result<(), sqlx::Error>
    {

        // On lance une transaction
        let mut tx: sqlx::Transaction<'_, sqlx::Sqlite> = pool.begin().await?;

        // 1. On remet l'état à zéro
        sqlx::query("UPDATE queue_state SET current_index = -1 WHERE profil_id = ?")
            .bind(profil_id)
            .execute(&mut *tx)
            .await?;

        // 2. On vide les pistes
        sqlx::query("DELETE FROM queue_tracks WHERE profil_id = ?")
            .bind(profil_id)
            .execute(&mut *tx)
            .await?;

        // On valide le tout !
        tx.commit().await?;

        Ok(())
    }

    pub async fn update_current_index<'e, E>(
        exec: E,
        profil_id: i64,
        current_index: i32,
    ) -> Result<(), sqlx::Error>
    where
        E: sqlx::Executor<'e, Database = sqlx::Sqlite>
    {

        sqlx::query(
            r#"
            UPDATE queue_state
            SET current_index = ?
            WHERE profil_id = ?
            "#
        )
        .bind(current_index)
        .bind(profil_id)
        .execute(exec)
        .await?;

        Ok(())
    }

    pub async fn update_is_shuffled<'e, E>(
        exec: E,
        profil_id: i64,
        is_shuffled: bool,
    ) -> Result<(), sqlx::Error>
    where
        E: sqlx::Executor<'e, Database = sqlx::Sqlite>
    {

        sqlx::query(
            r#"
            UPDATE queue_state
            SET is_shuffled = ?
            WHERE profil_id = ?
            "#
        )
        .bind(is_shuffled)
        .bind(profil_id)
        .execute(exec)
        .await?;

        Ok(())
    }

    pub async fn update_repeat_mode<'e, E>(
        exec: E,
        profil_id: i64,
        repeat_mode: &str,
    ) -> Result<(), sqlx::Error>
    where
        E: sqlx::Executor<'e, Database = sqlx::Sqlite>
    {

        sqlx::query(
            r#"
            UPDATE queue_state
            SET repeat_mode = ?
            WHERE profil_id = ?
            "#
        )
        .bind(repeat_mode)
        .bind(profil_id)
        .execute(exec)
        .await?;

        Ok(())
    }

}
