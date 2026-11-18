/// Colonne rendue par un composant plutôt que par du texte.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Widget {
    Text,
    Index,
    Cover,
    Title,
    Quality,
    Rating,
    Album,
}
