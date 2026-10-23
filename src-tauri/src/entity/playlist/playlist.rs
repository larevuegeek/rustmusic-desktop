use serde::{Deserialize, Serialize};
use sqlx::FromRow;

fn default_true() -> bool {
    true
}

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
    /// Playlist auto tirée au hasard chaque jour : un mix, pas une playlist.
    #[serde(default)]
    pub is_mix: bool,
    /// Affichée dans la barre latérale (épinglée).
    #[serde(default = "default_true")]
    pub pinned: bool,
    /// Playlist d'un serveur (synchronisée avec lui), sinon locale.
    #[serde(default)]
    #[sqlx(default)]
    pub server_id: Option<i64>,
    /// Source à laquelle elle appartient : 0 = « Cet ordinateur », sinon l'id du serveur.
    #[serde(default)]
    #[sqlx(default)]
    pub source_id: i64,
    pub created_at: String,
    pub updated_at: Option<String>,
}
