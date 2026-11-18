use crate::app::music_app::MusicApp;
use crate::commands::library::{composer_command, library_command};

impl MusicApp {
    /// Tous ses titres à la fin de la file.
    pub(crate) fn enqueue_artist(&mut self, artist_id: String) {
        let Some(library) = self.data.library_id else { return };
        let app = self.backend.app.clone();
        self.backend.call(
            async move { library_command::get_tracks_by_artist(app.state(), library, artist_id).await.unwrap_or_default() },
            |me, tracks| {
                me.enqueue_views(&tracks);
                true
            },
        );
    }

    /// Tous les titres de l'artiste, dans l'ordre de la discographie.
    pub(crate) fn play_artist(&mut self, artist_id: String) {
        let Some(library) = self.data.library_id else { return };
        let app = self.backend.app.clone();
        self.backend.call(
            async move { library_command::get_tracks_by_artist(app.state(), library, artist_id).await.unwrap_or_default() },
            |me, tracks| {
                if let Some(first) = tracks.first().map(|t| t.path.clone()) {
                    me.play_tracks_view(tracks, &first);
                }
                true
            },
        );
    }

    /// Les titres d'un compositeur (tag `composer`), lus ou ajoutés à la file.
    pub(crate) fn composer_tracks(&mut self, name: String, enqueue: bool) {
        let Some(library) = self.data.library_id else { return };
        let app = self.backend.app.clone();
        self.backend.call(
            async move { composer_command::get_tracks_by_composer(app.state(), library, name).await.unwrap_or_default() },
            move |me, tracks| {
                if enqueue {
                    me.enqueue_views(&tracks);
                } else if let Some(first) = tracks.first().map(|t| t.path.clone()) {
                    me.play_tracks_view(tracks, &first);
                }
                true
            },
        );
    }
}
