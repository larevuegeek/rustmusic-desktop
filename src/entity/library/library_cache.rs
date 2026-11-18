use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct LibraryCache {
    pub id: i64,
    pub path: String,

    pub title: Option<String>,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub album_artist: Option<String>,

    pub year: Option<String>,
    pub genre: Option<String>,
    pub track_number: Option<i32>,
    pub disc_number: Option<i32>,

    pub duration: Option<f64>,
    pub bitrate: Option<i32>,
    pub bits_per_sample: Option<i32>,
    pub sample_rate: Option<i32>,
    pub channels: Option<i32>,

    pub audio_format: Option<String>,
    pub mime_type: Option<String>,

    pub file_size: Option<i64>,

    pub extra_tags: Option<String>, // JSON string
    pub thumbnail_path: Option<String>,

    pub last_scanned_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LibraryCacheCreate {
    pub path: String,

    pub title: Option<String>,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub album_artist: Option<String>,

    pub year: Option<String>,
    pub genre: Option<String>,
    pub track_number: Option<i32>,
    pub disc_number: Option<i32>,

    pub duration: Option<f64>,
    pub bitrate: Option<i32>,
    pub bits_per_sample: Option<i32>,
    pub sample_rate: Option<i32>,
    pub channels: Option<i32>,

    pub audio_format: Option<String>,
    pub mime_type: Option<String>,

    pub file_size: Option<i64>,

    pub extra_tags: Option<String>, // JSON sérialisé
    pub thumbnail_path: Option<String>,
}

