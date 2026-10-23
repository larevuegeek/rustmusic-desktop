
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct RecentFileListView {
    pub id: i64,
    pub library_cache_id: Option<i64>,
    pub path: String,
    pub last_played_at: DateTime<Utc>,
    pub last_position: f64,
    pub play_count: i64,

    // Champs venant de library_cache (flat)
    pub cache_id: Option<i64>,
    pub cache_path: Option<String>,
    pub title: Option<String>,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub album_artist: Option<String>,
    pub year: Option<String>,
    pub genre: Option<String>,
    pub track_number: Option<i64>,
    pub disc_number: Option<i64>,
    pub duration: Option<f64>,
    pub bitrate: Option<i64>,
    pub bits_per_sample: Option<i64>,
    pub sample_rate: Option<i64>,
    pub channels: Option<i64>,
    pub audio_format: Option<String>,
    pub mime_type: Option<String>,
    pub file_size: Option<i64>,
    pub extra_tags: Option<String>,
    pub thumbnail_path: Option<String>,
    pub last_scanned_at: Option<DateTime<Utc>>,

    pub liked: bool,
}