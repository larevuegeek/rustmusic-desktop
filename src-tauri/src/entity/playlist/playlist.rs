use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Playlist {
    pub id: i64,
    pub profil_id: i64,
    pub library_id: Option<i64>,
    pub name: String,
    pub description: Option<String>,
    pub color: String,
    pub icon: String,
    pub cover: Option<String>,
    pub track_count: i64,
    pub duration: i64, // en secondes
    pub position: i64,
    /// Vrai si le contenu se calcule au lieu d'être rangé dans
    /// `playlist_items`. L'interface s'en sert pour proposer l'édition des
    /// règles plutôt que le retrait d'un morceau.
    #[serde(default)]
    pub is_smart: bool,
    pub created_at: String,
    pub updated_at: Option<String>,
}
