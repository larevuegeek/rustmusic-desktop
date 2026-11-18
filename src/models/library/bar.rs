use crate::models::app::Msg;

/// Une ligne de barres classées.
pub struct Bar {
    pub rank: usize,
    pub name: String,
    /// Valeur affichée en bout de ligne.
    pub count: String,
    /// Part du total (« 97 % »), pour les formats.
    pub share: Option<String>,
    /// Longueur de la barre, relative à la première.
    pub pct: f32,
    /// La fiche de la ligne (genre, artiste).
    pub open: Option<Msg>,
}
