/// Fenêtre glissante sur une liste déjà en mémoire : seuls les éléments `start..end` sont
/// bâtis. Elle grandit par pas à l'approche du bas et lâche une tranche en haut au-delà de
/// `capacity` ; l'ancrage du défilement garde l'élément regardé en place.
#[derive(Debug, Clone, PartialEq)]
pub struct ListWindow {
    /// Éléments gardés au plus.
    pub capacity: usize,
    pub start: usize,
    pub end: usize,
    /// Fin visée quand la fenêtre grandit par pas ; 0 : rien en cours.
    pub(crate) target_end: usize,
    /// Génération de la zone défilante : une nouvelle zone (filtres, tri, saut à une
    /// lettre) repart du haut, ou de `target`.
    pub view: u64,
    /// Index de l'élément amené en haut d'une nouvelle zone (saut à une lettre).
    pub target: Option<usize>,
}
