use serde::Serialize;
use sqlx::FromRow;

/// Ce qu'on a décidé pour un nom d'artiste douteux (« A, B », « A/B »…).
#[derive(Debug, Clone, FromRow, Serialize)]
pub struct ArtistSplitRule {
    pub name: String,
    /// whole | split | unknown
    pub decision: String,
    /// JSON : les artistes, ou la découpe proposée pour « unknown ».
    pub parts: Option<String>,
    /// tag | musicbrainz | user
    pub source: String,
    pub run_id: Option<i64>,
    pub updated_at: String,
}

impl ArtistSplitRule {
    pub fn parts_list(&self) -> Vec<String> {
        self.parts.as_deref().and_then(|p| serde_json::from_str(p).ok()).unwrap_or_default()
    }
}

/// Une vérification (complète ou d'un seul artiste).
#[derive(Debug, Clone, FromRow, Serialize)]
pub struct ArtistCheckRun {
    pub id: i64,
    pub started_at: String,
    pub finished_at: Option<String>,
    pub checked: i64,
    pub error: Option<String>,
}

/// Une piste à relier de nouveau à ses artistes.
#[derive(Debug, Clone, FromRow)]
pub struct ArtistRelinkRow {
    pub id: String,
    pub library_id: i64,
    pub artist_id: Option<String>,
    pub artist: Option<String>,
    /// AudioTags en JSON (pour le tag ARTISTS).
    pub tags: Option<String>,
    pub path: Option<String>,
}
