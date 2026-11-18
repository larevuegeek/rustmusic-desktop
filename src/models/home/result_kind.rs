/// Type d'un résultat de recherche.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResultKind {
    Track,
    Album,
    Artist,
    Composer,
}
