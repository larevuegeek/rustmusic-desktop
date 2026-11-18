use crate::models::app::Choice;
use crate::models::tracks::Table;

/// Ce que la barre d'outils et la liste lisent.
pub struct TracksView {
    pub table: Table,
    pub sort_label: String,
    pub sort_icon: &'static str,
    pub sorts: Vec<Choice>,
    pub densities: Vec<Choice>,
    pub genre_label: String,
    pub genres: Option<Vec<(String, String, bool)>>,
    pub loaded: String,
    pub letters: Vec<(&'static str, bool)>,
}
