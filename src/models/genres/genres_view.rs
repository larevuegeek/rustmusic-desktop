use crate::models::genres::*;
use crate::models::app::Choice;

pub struct GenresView {
    pub items: Vec<GenreItem>,
    /// Nom du genre amené en haut d'une nouvelle zone (saut à une lettre).
    pub target: Option<String>,
    pub untagged: i64,
    pub untagged_n: String,
    pub sort_label: String,
    pub sort_icon: &'static str,
    pub sorts: Vec<Choice>,
    pub views: Vec<Choice>,
    pub placeholder: String,
    pub letters: Vec<(&'static str, bool)>,
}
