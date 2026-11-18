use crate::models::albums::*;

#[derive(Debug, Clone, PartialEq)]
pub enum AlbumsMsg {
    Sort(String),
    Hires,
    Compilations,
    NoCover,
    GenreMenu,
    Genre(Option<String>),
    YearMenu,
    Year(Option<i32>),
    ClearFilters,
    View(ViewMode),
    Letter(String),
    /// Défilement de la grille : décalage et décalage maximal (fenêtre qui glisse).
    Scrolled(f32, f32),
    SearchSettled(u64),
    /// Lecture de l'album (bouton de la carte).
    Play(String),
    /// Ajout à la file (ligne de la liste).
    Enqueue(String),
}
