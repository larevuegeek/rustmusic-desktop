use serde::Serialize;

#[derive(Serialize, Clone)]
pub struct DirEntry {
    pub name: String,
    pub path: String,
    pub is_dir: bool,
    pub size: u64,
    pub extension: Option<String>,
    /// Ce que la bibliothèque sait du fichier (titre, artiste, durée, pochette) ; vide hors bibliothèque.
    pub title: Option<String>,
    pub artist: Option<String>,
    pub duration: Option<f64>,
    pub thumbnail_path: Option<String>,
}
