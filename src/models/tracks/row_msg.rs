/// Messages d'une ligne ou de l'en-tête.
#[derive(Debug, Clone, PartialEq)]
pub enum RowMsg {
    /// Le bouton de lecture (survol du numéro) : lecture, la file devient la liste.
    Play(usize),
    /// Un clic sur la ligne : lecture en mode « un clic », sinon désignation.
    Click(usize),
    /// Un double-clic sur la ligne : lecture, sauf en sélection et en mode « un clic »
    /// (le premier clic a déjà lancé le morceau).
    Open(usize),
    Like(usize),
    /// Note en demi-étoiles (0,5 à 5) ; la même note l'efface.
    Rate(usize, f64),
    /// Tri par colonne : premier clic croissant, le suivant inverse.
    Sort(String),
    /// Menu du morceau : clic droit (position) ou bouton « … » (son nom).
    Menu(usize, crate::models::tracks::TrackMenuAt),
}
