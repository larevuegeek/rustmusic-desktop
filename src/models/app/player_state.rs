use crate::models::app::*;
use crate::entity::audio::audio_file::AudioFile;

/// Le lecteur : morceau chargé, position, chaîne audio.
pub struct PlayerState {
    pub path: Option<String>,
    pub audio: Option<AudioFile>,
    pub status: Status,
    pub position: f64,
    pub duration: f64,
    /// `playback-preparing` : la barre de progression est figée.
    pub preparing: bool,
    /// `playback-pipeline` : source → sortie, tel que le moteur le décrit.
    pub pipeline: Option<serde_json::Value>,
    /// Morceau relancé sur une nouvelle sortie : la position où le reprendre, une fois prêt.
    pub seek_when_ready: Option<(String, f64)>,
    /// En pause quand la sortie a changé : la reprise relance le morceau sur la nouvelle.
    pub stale_output: bool,
}
