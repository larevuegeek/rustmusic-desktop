use crate::models::home::*;
use crate::models::app::Route;

/// Un résultat de recherche.
#[derive(Debug, Clone)]
pub struct SearchHit {
    pub title: String,
    pub subtitle: String,
    pub genre: ResultKind,
    pub cover_path: Option<String>,
    pub route: Option<Route>,
    pub path: Option<String>,
    /// Titre de la section qui commence à ce résultat (recherche rapide), vide sinon.
    pub section: String,
    /// Meilleur résultat : son type est rappelé par une étiquette.
    pub top: bool,
}
