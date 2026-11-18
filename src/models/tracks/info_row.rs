use crate::models::app::Route;

/// Une ligne d'une carte (`dl`) : libellé, valeur, lien, complément discret (« / 12 »).
pub struct InfoRow {
    pub label: String,
    pub value: String,
    pub route: Option<Route>,
    pub extra: Option<String>,
    /// Valeur estompée (aucune écoute).
    pub subtle: bool,
    /// Filet sous la ligne (pas sous la dernière rangée de la grille).
    pub ruled: bool,
}
