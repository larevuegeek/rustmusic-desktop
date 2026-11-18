use crate::models::app::Choice;
use crate::models::artists::ArtistItem;

pub struct ArtistsView {
    pub items: Vec<ArtistItem>,
    /// Élément amené en haut d'une nouvelle zone (`scroll-initial-target`).
    pub target: Option<String>,
    pub total: usize,
    pub shown_from: usize,
    pub shown_to: usize,
    pub sort_label: String,
    pub sort_icon: &'static str,
    pub sorts: Vec<Choice>,
    pub views: Vec<Choice>,
    pub placeholder: String,
    pub letters: Vec<(&'static str, bool)>,
    /// Bascule artistes / compositeurs : leurs nombres, si la bibliothèque a des compositeurs.
    pub modes: Option<(String, String)>,
    pub by_composer: bool,
}
