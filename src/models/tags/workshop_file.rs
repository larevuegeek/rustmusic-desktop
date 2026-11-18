#[derive(Debug, Clone)]
pub struct WorkshopFile {
    pub path: String,
    pub original: [String; 12],
    pub edits: [Option<String>; 12],
    pub readable: bool,
    pub cover: Option<String>,
    /// Pochette posée par « Compléter en ligne » (fichier, aperçu).
    pub pending_cover: Option<(String, String)>,
}
