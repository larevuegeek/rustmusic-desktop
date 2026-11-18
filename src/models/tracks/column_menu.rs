use crate::models::tracks::*;

/// Menu du clic droit sur l'en-tête d'une colonne.
#[derive(Debug, Clone, PartialEq)]
pub struct ColumnMenu {
    pub key: String,
    pub at: TrackMenuAt,
    /// Colonne qui prend la place libre (le titre) : sa largeur d'origine est automatique.
    pub flexible: bool,
    /// Colonne de texte, qu'on peut ajuster à son contenu.
    pub fit: bool,
    /// Colonne qu'on peut redimensionner (ni `#`, ni la pochette).
    pub resizable: bool,
}
