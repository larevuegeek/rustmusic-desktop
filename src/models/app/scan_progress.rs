use std::time::Instant;

/// Un scan ou un import en cours (`rescan-*`, `import-*`).
#[derive(Debug, Clone, Default)]
pub struct ScanProgress {
    pub current: i64,
    pub total: i64,
    pub label: String,
    /// Import d'un dossier (sinon rescan de la bibliothèque).
    pub import: bool,
    /// Bibliothèque visée.
    pub library_id: Option<i64>,
    /// Recherche des fichiers, avant l'analyse (`import-listing`).
    pub listing: bool,
    pub found: i64,
    pub directory: String,
    pub file_name: String,
    /// Nouveaux morceaux pas encore comptés par la bibliothèque.
    pub added: i64,
    pub started: Option<Instant>,
    /// Arrêt demandé, en attente de la fin.
    pub stopping: bool,
}
