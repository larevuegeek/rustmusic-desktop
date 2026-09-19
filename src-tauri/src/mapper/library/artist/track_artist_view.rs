use serde::{Deserialize, Serialize};
use sqlx::FromRow;

/// Un artiste crédité sur une piste. `library_artist_id` est `None` quand il
/// n'appartient pas à cette bibliothèque : son nom s'affiche alors sans lien.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct TrackArtistView {
    pub artist_id: String,
    pub library_artist_id: Option<String>,
    pub name: String,
}
