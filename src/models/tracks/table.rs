use crate::models::tracks::*;

/// La table prête à dessiner.
#[derive(Debug, Clone, Default)]
pub struct Table {
    pub head: Vec<HeadCell>,
    pub rows: Vec<Row>,
    pub detailed: bool,
    /// Aucune colonne n'absorbe la place : un remplissage après les colonnes.
    pub filler: bool,
    /// Ligne amenée en haut quand la zone défilante apparaît (`scroll-initial-target`).
    pub target: Option<usize>,
}
