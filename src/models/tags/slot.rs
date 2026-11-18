/// Une image en cours d'édition.
#[derive(Debug, Clone, PartialEq)]
pub struct Slot {
    /// Identifiant de l'image du fichier ; `None` pour une image ajoutée.
    pub id: Option<String>,
    /// Fichier d'une image ajoutée (Deezer).
    pub path: Option<String>,
    pub src: String,
    pub picture_type: u8,
    pub description: String,
    pub mime: String,
    pub bytes: usize,
}
