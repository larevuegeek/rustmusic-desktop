use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct ArtistListRow {
    pub id: String,
    pub name: String,
    pub total_albums: i64,
    pub total_tracks: i64,
    pub total_duration: f64,
    pub thumbnail_path: Option<String>, // future image artiste
}