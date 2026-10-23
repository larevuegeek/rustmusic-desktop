use serde::{Deserialize, Serialize};
use sqlx::FromRow;

/// D'où viennent les paroles. Stocké en TEXT, sérialisé en lowercase.
/// `None` = on a cherché et rien trouvé (évite de re-spammer LRCLIB).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[serde(rename_all = "lowercase")]
#[sqlx(rename_all = "lowercase")]
pub enum LyricsSource {
    Sidecar,
    Lrclib,
    Manual,
    /// Le serveur d'un morceau distant (paroles de ses fichiers).
    Server,
    None,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Lyrics {
    pub track_id: String,
    pub plain: Option<String>,
    pub synced: Option<String>,
    pub source: LyricsSource,
    pub fetched_at: i64,
    pub lrclib_id: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LyricsUpsert {
    pub track_id: String,
    pub plain: Option<String>,
    pub synced: Option<String>,
    pub source: LyricsSource,
    pub lrclib_id: Option<i64>,
}
