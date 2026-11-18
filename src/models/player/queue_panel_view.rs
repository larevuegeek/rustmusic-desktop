use crate::models::player::*;

/// Ce qu'affiche le volet de la file d'attente (`queue/QueuePanel.ui`), textes déjà mis
/// en forme.
pub struct QueuePanelView {
    /// La lecture est en cours (pastille animée, bouton pause, égaliseur).
    pub playing: bool,
    /// En pause : icône pause sur la carte du morceau en cours.
    pub paused: bool,
    /// Pastille d'état : « En lecture », « En pause »…
    pub status: String,
    /// « 12 titres ».
    pub count: String,
    /// Infobulle du bouton lecture / pause.
    pub toggle_label: String,
    /// La file contient des titres (bouton « Vider »).
    pub has_tracks: bool,
    pub now_playing: Option<QueueNowPlayingView>,
    /// « 11 à venir », absent quand rien ne suit le morceau en cours.
    pub upcoming_count: Option<String>,
    pub upcoming: Vec<QueueRowView>,
}
