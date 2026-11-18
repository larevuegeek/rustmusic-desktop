use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct TrackLikedRow {
    pub id: i64,
    pub liked_at: DateTime<Utc>, 
    pub path: String,
    pub library_cache_id: Option<i64>, 
    
    // --- Les champs issus de library_cache (lc) ---
    pub title: Option<String>,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub duration: Option<f64>, // ou i64, selon comment tu as défini ta colonne en DB
    pub thumbnail_path: Option<String>,
    pub bits_per_sample: Option<u32>, // ou i64 selon la DB
    pub audio_format: Option<String>,
    pub mime_type: Option<String>,
}