use crate::models::playlists::*;
use crate::app::music_app::MusicApp;
use karbon_ui::intl::t;
use crate::models::app::Msg;
use crate::models::app::Route;
use crate::models::library::CollectionSource;
use crate::models::layout::ToastKind;
use crate::models::playlists::PlaylistsMsg;
use crate::commands::playlist::playlist_command;

impl MusicApp {
    pub(crate) fn playlist_menu_update(&mut self, msg: PlaylistMenuMsg) -> bool {
        match msg {
            PlaylistMenuMsg::Open(at, id) => {
                let Some(p) = self.data.playlists.iter().find(|p| p.id == id) else { return false };
                self.menu = None;
                self.views.playlist_menu = Some(PlaylistMenu { at, id, name: p.name.clone(), smart: p.is_smart, pinned: p.pinned, server: p.server_id.is_some(), remote: p.source_id != 0, confirm: false });
            }
            PlaylistMenuMsg::Delete => {
                let Some(m) = self.views.playlist_menu.as_mut() else { return false };
                if m.server && !m.confirm {
                    m.confirm = true;
                    return true;
                }
                let Some(m) = self.views.playlist_menu.take() else { return false };
                let app = self.backend.app.clone();
                let (id, name) = (m.id, m.name);
                self.backend.call(async move { playlist_command::delete_playlist(app.state(), id).await }, move |me, r| {
                    match r {
                        Ok(_) => {
                            me.toast(ToastKind::Success, name, Some(t("playlist.deleted")));
                            me.load_playlists();
                            if *me.route() == Route::Playlist(id) {
                                return me.handle(Msg::Go(Route::Home));
                            }
                        }
                        Err(e) => me.toast(ToastKind::Error, t("notify.error"), Some(e)),
                    }
                    true
                });
            }
            PlaylistMenuMsg::Play => {
                let Some(m) = self.views.playlist_menu.take() else { return false };
                self.views.collection_menu = Some(crate::models::library::CollectionMenu {
                    at: m.at,
                    title: m.name,
                    source: CollectionSource::Playlist(m.id),
                    sub_playlists: false,
                    sub_cover: false,
                    pinned: None,
                });
                return self.collection_menu_update(crate::models::library::CollectionMenuMsg::PlayAll);
            }
            PlaylistMenuMsg::Pin => {
                let Some(m) = self.views.playlist_menu.take() else { return false };
                return self.handle(Msg::Playlists(PlaylistsMsg::Pin(m.id, !m.pinned)));
            }
            PlaylistMenuMsg::Edit => {
                let Some(m) = self.views.playlist_menu.take() else { return false };
                if m.smart {
                    return self.smart_update(crate::models::playlists::SmartMsg::Open(Some(m.id), false));
                }
                return self.playlist_form_update(crate::models::playlists::PlaylistFormMsg::Edit(m.id));
            }
            PlaylistMenuMsg::KeepOffline => {
                // Playlist d'un serveur : tous ses morceaux gardés hors ligne.
                let Some(m) = self.views.playlist_menu.take() else { return false };
                let app = self.backend.app.clone();
                self.backend.call(
                    async move {
                        let tracks_list = crate::commands::playlist::playlist_view_command::get_playlist_tracks_view(app.state(), m.id).await?;
                        let paths = tracks_list.into_iter().map(|p| p.path).filter(|p| !p.is_empty()).collect();
                        crate::commands::network::server_command::offline_add(app.clone(), app.state(), paths).await
                    },
                    |me, r| {
                        match r {
                            Ok(n) => {
                                let key = if n == 1 { "servers.offline_added_one" } else { "servers.offline_added_n" };
                                me.toast(ToastKind::Success, t("servers.offline_keep"), Some(t(key).replace("{n}", &n.to_string())));
                            }
                            Err(e) => me.toast(ToastKind::Error, t("notify.error"), Some(e)),
                        }
                        true
                    },
                );
            }
            PlaylistMenuMsg::Export | PlaylistMenuMsg::Close => self.views.playlist_menu = None,
        }
        true
    }
}
