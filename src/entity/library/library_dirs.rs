use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Clone, Default, Serialize, Deserialize, FromRow)]
pub struct LibraryDir {
    pub id: String,
    pub library_id: i64,
    
    pub path: String,
    pub name: String,
    
    pub is_recursive: bool,
    pub is_active: bool,
    pub watch_enabled: bool,
    
    pub include_patterns: Option<String>, // JSON string
    pub exclude_patterns: Option<String>, // JSON string
    
    pub total_files: i64,
    pub total_size: i64,
    pub last_scan_at: Option<DateTime<Utc>>,
    pub scan_status: String, // pending, scanning, completed, error
    pub scan_error: Option<String>,
    
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct LibraryDirCreate {
    pub library_id: i64,

    pub path: String,
    pub name: String,

    pub is_recursive: bool,
    pub is_active: bool,
    pub watch_enabled: bool,

    pub include_patterns: Option<String>, // JSON string
    pub exclude_patterns: Option<String>, // JSON string
}