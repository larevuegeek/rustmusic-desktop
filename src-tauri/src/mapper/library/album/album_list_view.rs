use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct AlbumListView {
    // ===== Identité Album =====
    pub id: String,
    pub library_id: i64,
    pub title: String,
    pub title_normalized: String,
    pub album_type: String, // album, single, ep, compilation
    pub musicbrainz_id: Option<String>,
    pub artist_id: Option<String>,
    pub artist: String, // nom uniquement pour listing
    pub year: Option<i32>,
    pub genre: Option<String>,
    pub cover_url: Option<String>,  // fallback éventuel depuis cache
    pub total_tracks: i64,
    pub total_duration: f64,
    pub notes: Option<String>,
    /// L'artiste n'est qu'invité sur cet album (compilation, BO, featuring).
    #[sqlx(default)]
    pub participation: bool,

    // ===== Timestamps =====
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
