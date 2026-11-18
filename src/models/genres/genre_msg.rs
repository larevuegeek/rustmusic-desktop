#[derive(Debug, Clone, PartialEq)]
pub enum GenreMsg {
    /// `albums`, `tracks` ou `artists`.
    View(&'static str),
    SortAlbums(&'static str),
    Sort(String),
    /// Cartes d'album suivantes ou précédentes (fenêtre de `CARDS`).
    /// Défilement de la fiche : décalage et décalage maximal (fenêtre qui glisse).
    Scrolled(f32, f32),
    /// Période : une année de la décennie, ou toute la décennie.
    Year(Option<i64>),
    PlayAll,
    Shuffle,
}
