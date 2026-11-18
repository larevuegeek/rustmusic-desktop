/// Un titre de l'onglet File du mini-lecteur.
pub struct MiniQueueRow {
    /// Rang dans la file entière (`MiniMsg::Play`).
    pub index: usize,
    /// Identifiant dans la file (`MiniMsg::Remove`, clé de la ligne).
    pub queue_id: String,
    pub title: String,
    /// L'artiste, vide s'il est inconnu.
    pub artist: String,
    pub cover: Option<String>,
    /// « 3:42 », absente sans durée connue.
    pub duration: Option<String>,
}
