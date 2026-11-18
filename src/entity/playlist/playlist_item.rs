use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct PlaylistItem {
    pub id: i64,
    pub playlist_id: i64,
    pub library_track_id: String,
    pub sort_index: i64,
    pub created_at: String,
}