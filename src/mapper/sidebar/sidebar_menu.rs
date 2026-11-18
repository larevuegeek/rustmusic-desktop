use crate::app::music_app::MusicApp;
use crate::models::sidebar::PinKind;

impl MusicApp {
    /// Titre de l'album ou de l'artiste en cours (`tags.album`, `tags.album_artist ?? tags.artist`).
    pub fn sidebar_playing_title(&self, kind: PinKind) -> String {
        let tags = self.data.player.audio.as_ref().map(|a| &a.tags);
        let tag = match kind {
            PinKind::Album => tags.and_then(|t| t.album.clone()),
            PinKind::Artist => tags.and_then(|t| t.album_artist.clone().or_else(|| t.artist.clone())),
        };
        tag.unwrap_or_default()
    }

    /// « Épingler l'album en cours » est inactif sans morceau en cours ou s'il l'est déjà.
    pub fn sidebar_pin_playing_disabled(&self, kind: PinKind) -> bool {
        match self.sidebar_playing(kind) {
            None => true,
            Some(id) => self.sidebar_pins(kind).iter().any(|p| p.id == id),
        }
    }
}
