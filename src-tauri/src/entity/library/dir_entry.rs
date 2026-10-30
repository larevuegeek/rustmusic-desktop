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
    /// Piste de bibliothèque correspondante, pour la sélection multiple.
    pub track_id: Option<String>,
    /// Sous-dossier : pochettes et qualité de ce qu'il contient.
    pub preview: Option<crate::service::library::folder_preview::FolderPreview>,
}
