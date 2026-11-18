/// Ce que devient le choix du dialogue en cours.
#[derive(Clone, Copy, Debug)]
pub enum Choice {
    PlayFile,
    PlayFolder,
    /// `handleAddFiles(libraryId, redirect)`.
    ImportFiles { library: i64, redirect: bool },
    /// `handleAddDirectory(libraryId, redirect)`.
    ImportFolder { library: i64, redirect: bool },
    /// Page d'import : la bibliothèque est créée après le choix.
    PageImport { folder: bool },
}
