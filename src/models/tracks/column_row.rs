/// Une colonne affichée, dans l'ordre (« Colonnes affichées »).
pub struct ColumnRow {
    /// Rang (`ColumnsMsg::Move`).
    pub index: usize,
    /// Numéro affiché, à partir de 1.
    pub rank: usize,
    /// Clé de la colonne (`ColumnsMsg::Toggle`).
    pub key: String,
    pub title: String,
    pub first: bool,
    pub last: bool,
}
