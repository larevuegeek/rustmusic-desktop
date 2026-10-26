use uuid::Uuid;

use crate::entity::library::library_album_artist::{
    LibraryAlbumArtist,
    LibraryAlbumArtistCreate,
};

pub struct LibraryAlbumArtistRepository;

impl LibraryAlbumArtistRepository {

    // ----------------------------------------------------
    // INSERT
    // ----------------------------------------------------
    pub async fn insert_library_album_artist<'e, E>(
        exec: E,
        data: LibraryAlbumArtistCreate,
    ) -> Result<LibraryAlbumArtist, sqlx::Error>
    where
        E: sqlx::Executor<'e, Database = sqlx::Sqlite>
    {

        let id: String = Uuid::new_v4().to_string();

        let result: LibraryAlbumArtist = sqlx::query_as::<_, LibraryAlbumArtist>(
            r#"
            INSERT INTO library_album_artists (
                id,
                library_id,
                library_album_id,
                artist_id
            )
            VALUES (?1, ?2, ?3, ?4)

            ON CONFLICT(library_album_id, artist_id)
            DO UPDATE SET
                library_id = excluded.library_id

            RETURNING *
            "#
        )
        .bind(id)
        .bind(data.library_id)
        .bind(data.library_album_id)
        .bind(data.artist_id)
        .fetch_one(exec)
        .await?;

        Ok(result)
    }

    // ----------------------------------------------------
    // FIND BY ALBUM
    // ----------------------------------------------------
    // ----------------------------------------------------
    // FIND BY ARTIST
    // ----------------------------------------------------
    // ----------------------------------------------------
    // DELETE ONE
    // ----------------------------------------------------

    // ----------------------------------------------------
    // DELETE ALL BY ALBUM
    // ----------------------------------------------------

}
