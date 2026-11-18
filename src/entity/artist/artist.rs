use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Artist {
    pub id: String,
    pub name: String,
    pub name_normalized: String,
    pub sort_name: String,
    pub bio: Option<String>,
    pub image_url: Option<String>,
    pub musicbrainz_id: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArtistCreate {
    pub name: String,
    pub name_normalized: String,
    pub sort_name: String
}