#[derive(Debug, Clone, PartialEq)]
pub enum TracksMsg {
    Sort(String),
    Reverse,
    Detailed(bool),
    Hires,
    Lossless,
    Favorites,
    NoCover,
    GenreMenu,
    Genre(Option<String>),
    ClearFilters,
    LoadNext,
    LoadPrevious,
    Letter(String),
    /// Défilement de la liste : décalage et décalage maximal.
    Scrolled(f32, f32),
    /// Fin du délai de saisie (300 ms) de la recherche numéro `n`.
    SearchSettled(u64),
}
