//! La liste des playlists : chargement des données et réponse aux messages.

use crate::models::playlists::*;
use crate::app::music_app::MusicApp;
use crate::commands::playlist::playlist_command;

impl MusicApp {
    pub(crate) fn playlists_search(&mut self, value: String) -> bool {
        self.views.playlists.search = value;
        true
    }

    pub(crate) fn playlists_update(&mut self, msg: PlaylistsMsg) -> bool {
        let page = &mut self.views.playlists;
        match msg {
            PlaylistsMsg::Kind(k) => page.kind = k,
            PlaylistsMsg::Sort(k) => {
                page.sort = k;
                self.menu = None;
            }
            PlaylistsMsg::ClearFilters => {
                page.search.clear();
                page.kind = "toutes";
            }
            PlaylistsMsg::Pin(id, pinned) => {
                if let Some(p) = self.data.playlists.iter_mut().find(|p| p.id == id) {
                    p.pinned = pinned;
                }
                let app = self.backend.app.clone();
                self.backend.fire(async move {
                    let _ = playlist_command::set_playlist_pinned(app.state(), id, pinned).await;
                });
            }
        }
        true
    }
}
