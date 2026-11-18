use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct LibraryArtist {
    pub id: String,
    pub library_id: i64,
    pub artist_id: String,
    
    pub total_albums: i64,
    pub total_tracks: i64,
    pub total_duration: f64,
    
    pub custom_image_url: Option<String>,
    pub notes: Option<String>,
    
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct LibraryArtistCreate {
    pub library_id: i64,
    pub artist_id: String,
}
