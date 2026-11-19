/// Messages de l'en-tête et du menu « Gérer » ; les actions de ce menu visent la
/// bibliothèque dont elles portent l'identifiant.
#[derive(Debug, Clone, PartialEq)]
pub enum LibraryMsg {
    /// Autre bibliothèque, même section.
    Switch(i64),
    /// Le bouton vert : action de la page (« Tout lire »…) ou « Ajouter un dossier ».
    Primary,
    /// Le menu « Gérer » d'une bibliothèque de la liste de la barre latérale.
    Manage(i64),
    /// Ferme le menu « Gérer » ouvert par-dessus la liste des bibliothèques.
    CloseManage,
    Rescan(i64),
    Folders(i64),
    AddFiles(i64),
    Covers(i64),
    Portraits,
    CheckArtists,
    ImportExport,
    Rename(i64),
    MakeDefault(i64),
    Delete(i64),
}
