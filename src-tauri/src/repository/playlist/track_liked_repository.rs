use crate::{entity::playlist::track_liked::TrackLiked, mapper::playlist::liked::track_liked_view::TrackLikedView};

pub struct TrackLikedRepository;

impl TrackLikedRepository {

    pub async fn insert_track_liked<'e, E>(
        exec: E,
        path: &str,
        profil_id: i64,
        library_cache_id: Option<i64>
    ) -> Result<TrackLiked, sqlx::Error>
    where
        E: sqlx::Executor<'e, Database = sqlx::Sqlite>
    {

        let result: TrackLiked = sqlx::query_as::<_, TrackLiked>(
            r#"
            INSERT INTO track_liked (
                path,
                profil_id,
                library_cache_id,
                created_at
            )
            VALUES (?1, ?2, ?3, CURRENT_TIMESTAMP)

            ON CONFLICT(path, profil_id)
            DO NOTHING

            RETURNING *
            "#
        )
        .bind(path)
        .bind(profil_id)
        .bind(library_cache_id)
        .fetch_one(exec)
        .await?;

        Ok(result)

    }


    pub async fn find_all_by_profile<'e, E>(
        exec: E,
        profil_id: i64,
    ) -> Result<Vec<TrackLikedView>, sqlx::Error>
    where
        E: sqlx::Executor<'e, Database = sqlx::Sqlite>
    {
        let results: Vec<TrackLikedView> = sqlx::query_as::<_, TrackLikedView>(
            r#"
                SELECT
                tl.id,
                tl.created_at  AS liked_at,
                tl.path,
                lc.id          AS library_cache_id,
                lc.title,
                lc.artist,
                lc.album,
                lc.duration,
                lc.thumbnail_path,
                lc.bits_per_sample,
                lc.audio_format,
                lc.mime_type
                FROM track_liked tl
                LEFT JOIN library_cache lc ON lc.id = tl.library_cache_id
                WHERE profil_id = ?1
                ORDER BY tl.created_at DESC
            "#
        )
        .bind(profil_id)
        .fetch_all(exec)
        .await?;

        Ok(results)
    }

    pub async fn remove_track_liked<'e, E>(
        exec: E,
        path: &str,
        profil_id: i64
    ) -> Result<(), sqlx::Error>
    where
        E: sqlx::Executor<'e, Database = sqlx::Sqlite>
    {

        sqlx::query(
                    r#"
                    DELETE FROM track_liked
                    WHERE path = ?1
                    AND profil_id = ?2
                    "#
                )
                .bind(path)
                .bind(profil_id)
                .execute(exec)
                .await?;

        Ok(())
    }

}
