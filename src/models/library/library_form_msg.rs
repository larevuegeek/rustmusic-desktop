#[derive(Debug, Clone, PartialEq)]
pub enum LibraryFormMsg {
    Create,
    /// Renommer la bibliothèque ouverte.
    Rename,
    Name(String),
    Desc(String),
    Submit,
    /// « Supprimer la bibliothèque » (l'ouverte), puis la suppression confirmée.
    AskDelete,
    Delete(i64),
}
