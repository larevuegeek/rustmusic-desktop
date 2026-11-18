//! Le lecteur de la zone de notification : l'état à lui envoyer.

use crate::app::music_app::MusicApp;
use crate::models::tray::TraySnapshot;

impl MusicApp {
    /// Morceau courant, lecture et avancement (au centième : le lecteur n'est prévenu que
    /// quand la barre bouge à l'œil).
    pub(crate) fn tray_snapshot(&self) -> TraySnapshot {
        let Some(track) = self.data.current_track() else {
            return TraySnapshot::default();
        };
        let duration = track.duration.filter(|d| *d > 0.).unwrap_or(0.);
        let progress = if duration > 0. { (self.data.player.position / duration).clamp(0., 1.) } else { 0. };
        TraySnapshot {
            has_track: true,
            title: track.title.clone(),
            artist: track.artist.clone().unwrap_or_default(),
            cover: track.cover.clone(),
            playing: self.playing(),
            progress: ((progress * 100.).round() / 100.) as f32,
        }
    }
}
