#[derive(Debug, Clone, PartialEq)]
pub enum LibraryFoldersMsg {
    Open,
    /// Import d'un dossier dans la bibliothèque.
    Add,
    Close,
    Rescan(String),
    Remove(String),
}
