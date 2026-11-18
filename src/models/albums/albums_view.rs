use crate::models::albums::*;
use crate::models::app::Choice;

pub struct AlbumsView {
    pub items: Vec<AlbumItem>,
    /// Élément amené en haut d'une nouvelle zone (`scroll-initial-target`).
    pub target: Option<String>,
    pub total: usize,
    pub shown_from: usize,
    pub shown_to: usize,
    pub sort_label: String,
    pub sort_icon: &'static str,
    pub sorts: Vec<Choice>,
    pub views: Vec<Choice>,
    pub genres: Vec<(String, String, bool)>,
    pub genre_label: String,
    pub years: Vec<(i32, String, bool)>,
    pub placeholder: String,
    pub empty: String,
    pub letters: Vec<(&'static str, bool)>,
}
