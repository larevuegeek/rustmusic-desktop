#[derive(Debug, Clone, PartialEq)]
pub enum ArtistCheckMsg {
    /// Lancer sur toute la bibliothèque (ouvre la fenêtre).
    Start,
    /// Ouvrir le bilan de la dernière vérification.
    Report,
    /// Un seul nom (menu d'un artiste épinglé) : la fenêtre l'affiche en cours,
    /// puis son bilan.
    One(String),
    Close,
    Undo(String),
    Redo(String),
    UndoAll,
    Decide(String, bool),
}
