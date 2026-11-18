/// Un maillon du fil d'Ariane du sélecteur de dossiers DLNA.
pub struct DlnaCrumb {
    /// Rang dans le chemin (`DlnaMsg::Trail`) ; 0 pour la racine.
    pub index: usize,
    /// Le nom du dossier, ou « Racine ».
    pub label: String,
    /// Le dossier ouvert : pas de lien.
    pub current: bool,
}
