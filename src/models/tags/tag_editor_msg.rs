#[derive(Debug, Clone, PartialEq)]
pub enum TagEditorMsg {
    Open(String),
    Field(usize, String),
    Revert(usize),
    RevertAll,
    SetCover(usize),
    Move(usize, bool),
    Remove(usize),
    Zoom(Option<usize>),
    Fetch,
    Submit,
    /// Fermeture demandée (croix, fond, Annuler) : refusée tant qu'il reste des changements.
    Close,
    Discard,
    KeepEditing,
}
