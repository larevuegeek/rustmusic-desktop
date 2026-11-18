use serde::Serialize;
use sqlx::FromRow;

#[derive(Debug, Serialize, Clone, FromRow)]
pub struct SearchResult {
    pub id: String,
    pub result_type: String,
    pub title: String,
    pub subtitle: Option<String>,
    pub thumbnail_path: Option<String>,
    pub path: Option<String>,
    pub library_id: Option<i64>,
}
