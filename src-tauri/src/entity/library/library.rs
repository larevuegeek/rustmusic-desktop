
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Library {
    pub id: i64,
    pub profil_id: i64,
    pub name: String,
    pub description: Option<String>,
    pub cover: Option<String>,
    pub position: i64,
    pub total_tracks: i64,
    pub total_albums: i64,
    pub total_artists: i64,
    /// Bibliothèque ouverte au démarrage. Au plus une par profil — l'unicité
    /// est tenue par un index partiel, pas par le code appelant.
    pub is_default: bool,
    pub created_at: String,
    pub updated_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct LibraryCreate {
    pub profil_id: i64,
    pub name: String,
    pub description: Option<String>
}