#[derive(Debug, Clone, PartialEq)]
pub enum ArtistsMsg {
    Sort(String),
    NoPortrait,
    ClearFilters,
    /// « artists » ou « composers ».
    Mode(&'static str),
    Letter(String),
    /// Défilement de la grille : décalage et décalage maximal (fenêtre qui glisse).
    Scrolled(f32, f32),
    SearchSettled(u64),
}
