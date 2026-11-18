use crate::models::player::*;

/// L'onglet File du mini-lecteur (`mini/MiniQueue.ui`), textes déjà mis en forme.
pub struct MiniQueueView {
    pub now_playing: Option<MiniQueueRow>,
    /// « Ensuite · 3 titres ».
    pub next_label: String,
    /// « Vider la suite » : un morceau en cours et des titres après lui.
    pub can_clear: bool,
    /// Les titres à venir bâtis (fenêtre qui glisse) ; vide : « Aucun morceau à suivre ».
    pub rows: Vec<MiniQueueRow>,
}
