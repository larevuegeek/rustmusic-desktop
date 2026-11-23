use crate::models::library::*;
use crate::app::music_app::MusicApp;
use karbon_ui::intl::t;

impl CollectionMenu {
    pub fn play_all_label(&self) -> String {
        t(match self.source {
            CollectionSource::Album { .. } => "menu.play_all_album",
            CollectionSource::Genre(_) => "menu.play_all_genre",
            CollectionSource::Composer(_) => "menu.play_all_composer",
            CollectionSource::Period(..) => "artist_view.play_all",
            CollectionSource::Playlist(_) => "menu.play_all_playlist",
        })
    }

    pub fn is_album(&self) -> bool {
        matches!(self.source, CollectionSource::Album { .. })
    }
}

impl MusicApp {
    /// Les playlists qui acceptent les morceaux de la bibliothèque.
    pub fn collection_menu_playlists(&self) -> Vec<&crate::entity::playlist::playlist::Playlist> {
        let server = self.data.libraries.iter().find(|l| Some(l.id) == self.data.library_id).and_then(|l| l.server_id);
        self.data.playlists.iter().filter(|p| !p.is_smart && p.server_id == server).collect()
    }
}
