use crate::models::search::*;
use crate::models::app::Choice;

pub struct SearchView {
    pub query: String,
    pub counts: Vec<(String, String)>,
    pub has_tracks: bool,
    pub empty: bool,
    pub loading: bool,
    pub filtres: Vec<Choice>,
    pub filtre: &'static str,
    pub top: Option<Hit>,
    pub tracks: Vec<Hit>,
    pub albums: Vec<Hit>,
    pub artists: Vec<Hit>,
    pub composers: Vec<Hit>,
    /// En-têtes des sections : morceaux à côté du meilleur résultat (5 montrés) ou seuls
    /// (8), albums, artistes et compositeurs (6).
    pub top_head: SearchSection,
    pub tracks_beside_top: SearchSection,
    pub tracks_head: SearchSection,
    pub albums_head: SearchSection,
    pub artists_head: SearchSection,
    pub composers_head: SearchSection,
}
