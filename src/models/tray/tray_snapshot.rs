/// Ce que montre le lecteur de la zone de notification, envoyé par la fenêtre principale
/// quand il change.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct TraySnapshot {
    /// Un morceau est chargé (sinon : « Rien en lecture »).
    pub has_track: bool,
    pub title: String,
    pub artist: String,
    pub cover: Option<String>,
    pub playing: bool,
    /// Avancement, de 0 à 1.
    pub progress: f32,
}
