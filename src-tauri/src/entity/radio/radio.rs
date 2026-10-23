use serde::Serialize;
use sqlx::FromRow;

/// Une radio favorite du profil.
#[derive(Debug, Clone, Serialize, FromRow)]
pub struct RadioView {
    pub id: i64,
    pub uuid: Option<String>,
    pub name: String,
    pub stream_url: String,
    pub homepage: Option<String>,
    pub favicon_url: Option<String>,
    pub logo_path: Option<String>,
    pub country_code: Option<String>,
    /// Genres séparés par des virgules.
    pub tags: String,
    pub codec: Option<String>,
    pub bitrate: Option<i64>,
    pub reduced_quality: bool,
    pub position: i64,
    pub last_played_at: Option<String>,
    pub created_at: String,
}

/// Un titre passé à l'antenne pendant l'écoute.
#[derive(Debug, Clone, Serialize, FromRow)]
pub struct RadioTitle {
    pub id: i64,
    pub station: String,
    pub stream_url: String,
    pub title: String,
    pub played_at: String,
    /// Logo de la station, s'il est en cache (radios récentes).
    pub logo_path: Option<String>,
}

/// Une radio écoutée récemment (favorite ou non).
#[derive(Debug, Clone, Serialize, FromRow)]
pub struct RadioRecent {
    pub id: i64,
    pub uuid: Option<String>,
    pub name: String,
    pub stream_url: String,
    pub homepage: Option<String>,
    pub favicon_url: Option<String>,
    pub logo_path: Option<String>,
    pub country_code: Option<String>,
    pub tags: String,
    pub codec: Option<String>,
    pub bitrate: Option<i64>,
    pub reduced_quality: bool,
    pub played_at: String,
}
