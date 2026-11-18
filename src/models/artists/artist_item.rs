use crate::models::app::Msg;
use crate::models::app::Route;

/// Une carte (grille) ou ligne (liste) d'artiste ou de compositeur.
#[derive(Debug, Clone)]
pub struct ArtistItem {
    pub id: String,
    pub name: String,
    pub portrait: Option<String>,
    /// `ArtistAvatar` (compositeur sans photo) : initiales et teinte tirée du nom.
    pub avatar: Option<(String, f32)>,
    /// « 3 albums · 41 titres ».
    pub counts: String,
    pub albums_n: String,
    pub albums_label: String,
    pub tracks_n: String,
    pub tracks_label: String,
    pub duration: String,
    pub route: Route,
    pub play: Msg,
    pub enqueue: Msg,
}
