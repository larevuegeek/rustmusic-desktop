/// Un titre à venir dans le volet de la file.
pub struct QueueRowView {
    /// Rang dans la file entière (`Msg::PlayQueue`).
    pub index: usize,
    /// Numéro affiché : 1 pour le titre suivant.
    pub rank: usize,
    /// Identifiant dans la file (`Msg::QueueRemove`, clé de la ligne).
    pub queue_id: String,
    pub title: String,
    /// L'artiste, ou « Artiste inconnu ».
    pub artist: String,
    pub cover: Option<String>,
    /// « 03:42 ».
    pub duration: String,
}
