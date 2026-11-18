use crate::models::app::Msg;

/// Une carte (grille) ou ligne (liste) d'album.
#[derive(Debug, Clone)]
pub struct AlbumItem {
    pub id: String,
    pub title: String,
    pub artist: String,
    pub year: String,
    pub cover: Option<String>,
    pub hires: Option<String>,
    pub tracks: String,
    pub tracks_n: String,
    pub tracks_label: String,
    pub duration: String,
    pub compilation: bool,
    /// Clic sur la carte : la page de l'album (la discographie d'un artiste ouvre son groupe).
    pub open: Msg,
}
