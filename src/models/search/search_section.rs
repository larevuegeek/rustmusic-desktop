use crate::models::app::Msg;

/// En-tête d'une section de résultats (`search/Section.ui`).
pub struct SearchSection {
    pub title: String,
    /// Nombre de résultats, absent à zéro.
    pub count: Option<String>,
    /// « Voir tous les résultats », quand la section n'en montre qu'une partie.
    pub see_all: Option<Msg>,
}
