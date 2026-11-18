use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct LibraryAlbum {
    pub id: String,
    pub artist_id: String,
    pub library_id: i64,

    pub title: String,
    pub title_normalized: String,

    pub year: Option<i32>,
    pub genre: Option<String>,
    pub cover_url: Option<String>,
    pub musicbrainz_id: Option<String>,
    pub album_type: String, // album, single, compilation, ep
    
    pub total_tracks: i64,
    pub total_duration: f64,
    
    pub notes: Option<String>,
    
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct LibraryAlbumCreate {
    pub library_id: i64,
    pub artist_id: String,
    /// Le dossier qui identifie l'album, disques multiples rassemblés.
    pub album_dir: String,
    
    pub title: String,
    pub title_normalized: String,
    
    pub year: Option<i32>,
    pub genre: Option<String>,
    pub cover_url: Option<String>,
    pub album_type: Option<String>,
}

/// Une piste d'un album pas encore rangé par dossier (`album_dir` vide), pour le rassemblement.
#[derive(Debug, FromRow)]
pub struct AlbumMergeRow {
    pub album_id: String,
    pub library_id: i64,
    pub title_normalized: String,
    pub artist_id: String,
    pub cover_url: Option<String>,
    pub path: Option<String>,
    pub track_artist: Option<String>,
    pub tag_album: Option<String>,
}

/// Une piste d'un album distant, pour réunir les compilations éclatées par le serveur.
#[derive(Debug, FromRow)]
pub struct RemoteAlbumRow {
    pub album_id: String,
    pub title_normalized: String,
    pub year: Option<i64>,
    pub cover_url: Option<String>,
    pub disc: Option<i64>,
    pub track: Option<i64>,
    pub tag_album: Option<String>,
}
