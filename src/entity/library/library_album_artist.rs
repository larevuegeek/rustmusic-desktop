use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct LibraryAlbumArtist {
    pub id: String,
    pub library_id: i64,
    pub library_album_id: String,
    pub artist_id: String,
    pub role: Option<String>, // primary | featured | composer | etc
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LibraryAlbumArtistCreate {
    pub library_id: i64,
    pub library_album_id: String,
    pub artist_id: String,
}
