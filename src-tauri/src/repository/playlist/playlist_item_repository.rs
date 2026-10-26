use sqlx::Error;

use crate::{entity::playlist::playlist_item::PlaylistItem, mapper::playlist::playlist::playlist_track_view::PlaylistTrackView};

pub struct PlaylistItemRepository;

impl PlaylistItemRepository {

 pub async fn insert_playlist_item<'e, E>(
        exec: E,
        playlist_id: i64,
        library_track_id: String,
        sort_index: i64,
    ) -> Result<PlaylistItem, Error>
    where
        E: sqlx::Executor<'e, Database = sqlx::Sqlite>
    {
        let item = sqlx::query_as::<_, PlaylistItem>(
            r#"
            INSERT INTO playlist_items (
                playlist_id,
                library_track_id,
                sort_index
            )
            VALUES (?, ?, ?)
            RETURNING
                id,
                playlist_id,
                library_track_id,
                sort_index,
                created_at
            "#,
        )
        .bind(playlist_id)
        .bind(library_track_id)
        .bind(sort_index)
        .fetch_one(exec)
        .await?;

        Ok(item)
    }


    pub async fn find_by_id<'e, E>(exec: E, id: i64) -> Result<Option<PlaylistItem>, Error>
    where
        E: sqlx::Executor<'e, Database = sqlx::Sqlite>
    {
        let item = sqlx::query_as::<_, PlaylistItem>(
            r#"
            SELECT
                id,
                playlist_id,
                library_track_id,
                sort_index,
                created_at
            FROM playlist_items
            WHERE id = ?
            "#,
        )
        .bind(id)
        .fetch_optional(exec)
        .await?;

        Ok(item)
    }

    pub async fn find_all_tracks_by_playlist_id<'e, E>(
        exec: E,
        playlist_id: i64,
    ) -> Result<Vec<PlaylistTrackView>, Error>
    where
        E: sqlx::Executor<'e, Database = sqlx::Sqlite>
    {
        let items = sqlx::query_as::<_, PlaylistTrackView>(
            r#"
            SELECT
                pi.id                AS playlist_item_id,
                pi.playlist_id       AS playlist_id,
                pi.sort_index        AS sort_index,

                lt.id                AS library_track_id,
                lt.title             AS title,
                lt.duration          AS duration,
                lt.play_count        AS play_count,
                lt.track_number      AS track_number,
                lt.disc_number       AS disc_number,

                la.id                AS album_id,
                la.title             AS album_title,

                ar.id                AS artist_id,
                ar.name              AS artist_name,

                lf.path              AS path,
                lc.thumbnail_path    AS thumbnail_path
            FROM playlist_items pi
            INNER JOIN library_tracks lt
                ON lt.id = pi.library_track_id
            LEFT JOIN library_files lf
                ON lf.id = lt.file_id
            LEFT JOIN library_cache lc
                ON lc.id = lt.cache_id
            LEFT JOIN library_albums la
                ON la.id = lt.library_album_id
            LEFT JOIN artists ar
                ON ar.id = lt.artist_id
            WHERE pi.playlist_id = ?
            ORDER BY pi.sort_index ASC, pi.id ASC
            "#,
        )
        .bind(playlist_id)
        .fetch_all(exec)
        .await?;

        Ok(items)
    }



    pub async fn get_next_sort_index<'e, E>(exec: E, playlist_id: i64) -> Result<i64, Error>
    where
        E: sqlx::Executor<'e, Database = sqlx::Sqlite>
    {
        let next_index: i64 = sqlx::query_scalar(
            r#"
            SELECT COALESCE(MAX(sort_index), -1) + 1
            FROM playlist_items
            WHERE playlist_id = ?
            "#,
        )
        .bind(playlist_id)
        .fetch_one(exec)
        .await?;

        Ok(next_index)
    }

    pub async fn delete_playlist_item<'e, E>(exec: E, id: i64) -> Result<u64, Error>
    where
        E: sqlx::Executor<'e, Database = sqlx::Sqlite>
    {
        let result = sqlx::query(
            r#"
            DELETE FROM playlist_items
            WHERE id = ?
            "#,
        )
        .bind(id)
        .execute(exec)
        .await?;

        Ok(result.rows_affected())
    }

}
