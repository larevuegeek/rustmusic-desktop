#[derive(Debug, Clone, PartialEq)]
pub enum GenresMsg {
    Sort(String),
    Main,
    Untagged,
    /// La carte « Sans genre » : la vue Albums filtrée sur les albums sans genre.
    OpenUntagged,
    ClearFilters,
    Letter(String),
    /// Défilement de la liste : décalage et décalage maximal (fenêtre qui glisse).
    Scrolled(f32, f32),
    Play(String, bool),
}
