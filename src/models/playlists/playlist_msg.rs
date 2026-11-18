#[derive(Debug, Clone, PartialEq)]
pub enum PlaylistMsg {
    Sort(String),
    PlayAll,
    Shuffle,
    Pin,
    AskDelete(bool),
    Delete,
    ClearHistory,
    /// Retire le morceau (rang dans l'ordre affiché) de la playlist, des likes.
    Remove(usize),
}
