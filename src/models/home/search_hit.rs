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
}
