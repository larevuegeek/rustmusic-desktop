use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct LibraryTrackArtist {
    pub id: String,
    pub library_id: i64,
    pub library_track_id: String,
    pub artist_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LibraryTrackArtistCreate {
    pub library_id: i64,
    pub library_track_id: String,
    pub artist_id: String,
}
