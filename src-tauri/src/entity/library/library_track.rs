use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct LibraryTrack {
    pub id: String,
    pub library_id: i64,
    pub file_id: String,
    pub cache_id: Option<i64>,
    
    pub artist_id: Option<String>,
    pub library_album_id: Option<String>,
    
    pub title: String,
    pub title_normalized: String,
    pub track_number: Option<i32>,
    pub disc_number: i32,
    
    pub tags: Option<String>, // JSON array
    
    pub duration: Option<f64>,
    pub bitrate: Option<i32>,
    pub sample_rate: Option<i32>,
    
    pub play_count: i64,
    pub last_played_at: Option<DateTime<Utc>>,
    pub rating: Option<f64>,
    pub favorite: bool,
    
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LibraryTrackCreate {
    pub library_id: i64,
    pub file_id: String,
    pub cache_id: Option<i64>,

    pub artist_id: Option<String>,
    pub library_album_id: Option<String>,

    pub title: String,
    pub title_normalized: String,
    pub track_number: Option<i32>,
    pub disc_number: i32,

    pub tags: Option<String>, // JSON (cache / mémoire)

    pub duration: Option<f64>,
    pub bitrate: Option<i32>,
    pub sample_rate: Option<i32>,
    pub rating: Option<f64>,
}

/// Une piste, son artiste principal et son tag « artiste » brut (reprise des liaisons).
#[derive(Debug, sqlx::FromRow)]
pub struct TrackArtistTagRow {
    pub id: String,
    pub library_id: i64,
    pub artist_id: Option<String>,
    pub artist: Option<String>,
}

/// Où trouver, en bibliothèque, le morceau joué depuis ce chemin.
///
/// Le lecteur ne connaît que le fichier : sans ce pont, son titre, son artiste
/// et son album ne mènent nulle part. Un chemin absent de la bibliothèque —
/// fichier ouvert à la volée — ne rend simplement rien.
#[derive(Debug, Clone, serde::Serialize, sqlx::FromRow)]
pub struct TrackLocation {
    pub path: String,
    pub library_id: i64,
    pub library_track_id: String,
    pub artist_id: Option<String>,
    pub library_album_id: Option<String>,
}
