use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct TrackLiked {
    pub id: i64,
    pub library_cache_id: Option<i64>,
    pub path: String,
    pub created_at: DateTime<Utc>,
}
