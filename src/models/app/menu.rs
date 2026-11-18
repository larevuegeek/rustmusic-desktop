/// Menus, popins et panneaux de l'application.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Menu {
    Timer,
    Profile,
    Source,
    /// « … » de Fichier / Dossier
    Open,
    Library,
    /// Options d'affichage des playlists de la barre
    Display,
    /// « … » d'une carte de mix, par position dans « Pour vous »
    Mix(usize),
    /// Sortie audio
    Output,
    /// File d'attente
    File,
    /// Paroles
    Lyrics,
    /// Lecteurs réseau
    Network,
    /// Nouveautés
    WhatsNew,
    /// Suggestions de recherche
    Search,
    /// Menu d'une page, par l'identifiant de son déclencheur.
    Popup(&'static str),
}
