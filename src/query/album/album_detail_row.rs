use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct AlbumDetailRow {
    pub id: String,
    pub library_id: i64,
    pub title: String,
    pub title_normalized: String,
    pub album_type: String, // album | single | ep | compilation
    pub musicbrainz_id: Option<String>,
    pub artist_id: String, // utile pour navigation
    pub artist: String,    // nom affiché
    pub year: Option<i32>,
    pub genre: Option<String>,
    pub notes: Option<String>,
    pub cover_url: Option<String>,       // cover officielle album
    pub thumbnail_path: Option<String>,  // fallback cache track si besoin
    pub total_tracks: i64,
    pub total_duration: f64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}