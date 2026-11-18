use crate::models::library::*;
use karbon_ui::intl;
use crate::app::music_app::MusicApp;
use karbon_ui::intl::t;
use crate::models::app::Msg;
use crate::models::app::Route;
use crate::models::tracks::TrackMenuAt;
use crate::models::layout::ToastKind;
use crate::commands::library::{library_command, pin_command};
use crate::commands::playlist::playlist_command;
use crate::query::track::track_list_row::TrackListRow;

/// « 3 morceaux ajoutés à … » : clé `_one` ou `_n`.
pub(crate) fn count(n: usize, one: &str, many: &str) -> String {
    t(if n == 1 { one } else { many }).replace("{n}", &intl::number(n as i64))
}

impl MusicApp {
    pub(crate) fn collection_menu_open(&mut self, at: TrackMenuAt, title: String, source: CollectionSource) -> bool {
        self.menu = None;
        self.views.track_menu = None;
        let album = match &source {
            CollectionSource::Album { id, .. } => Some(id.clone()),
            _ => None,
        };
        self.views.collection_menu = Some(CollectionMenu { at, title, source, sub_playlists: false, sub_cover: false, pinned: None });
        if let (Some(album), Some(library)) = (album, self.data.library_id) {
            let app = self.backend.app.clone();
            self.backend.call(async move { pin_command::get_library_pins(app.state(), library).await }, move |me, r| {
                let Some(m) = me.views.collection_menu.as_mut() else { return false };
                m.pinned = Some(r.map(|l| l.iter().any(|p| p.kind == "album" && p.id == album)).unwrap_or(false));
                true
            });
        }
        true
    }

    /// Charge les morceaux, puis fait l'action.
    pub(crate) fn collection_with_tracks(&mut self, source: CollectionSource, then: impl FnOnce(&mut MusicApp, Vec<TrackListRow>) -> bool + Send + 'static) {
        let app = self.backend.app.clone();
        let library = self.data.library_id.unwrap_or(0);
        let album = matches!(source, CollectionSource::Album { .. });
        self.backend.call(
            async move {
                match source {
                    CollectionSource::Album { id, .. } => library_command::get_tracks_by_album(app.state(), library, id).await.unwrap_or_default(),
                    CollectionSource::Genre(g) => library_command::get_tracks_by_genre(app.state(), library, g).await.unwrap_or_default(),
                    CollectionSource::Playlist(id) => crate::commands::playlist::playlist_view_command::get_playlist_tracks_view(app.state(), id).await.unwrap_or_default(),
                }
            },
            move |me, mut tracks_list| {
                if album {
                    tracks_list.sort_by(crate::mapper::albums::album_detail::disc_order);
                }
                if tracks_list.is_empty() {
                    return false;
                }
                then(me, tracks_list)
            },
        );
    }

    pub(crate) fn collection_menu_update(&mut self, msg: CollectionMenuMsg) -> bool {
        let Some(m) = self.views.collection_menu.as_mut() else { return false };
        match msg {
            CollectionMenuMsg::SubPlaylists => {
                m.sub_playlists = !m.sub_playlists;
                return true;
            }
            CollectionMenuMsg::SubCover => {
                m.sub_cover = !m.sub_cover;
                return true;
            }
            _ => {}
        }
        let Some(m) = self.views.collection_menu.take() else { return false };
        let profile = self.data.profile_id().unwrap_or(0);
        match msg {
            CollectionMenuMsg::PlayAll => self.collection_with_tracks(m.source, move |me, tracks_list| {
                let file = crate::controller::tracks::track_rows::to_queue(profile, &tracks_list);
                let first = tracks_list[0].path.clone();
                me.play_list(file, &first);
                true
            }),
            CollectionMenuMsg::PlayNext => self.collection_with_tracks(m.source, move |me, tracks_list| {
                // Insérés un à un après le morceau en cours : le dernier ajouté passe en premier.
                for q in crate::controller::tracks::track_rows::to_queue(profile, &tracks_list).into_iter().rev() {
                    me.queue_add_next(q);
                }
                me.toast(ToastKind::Success, t("notify.added_next"), Some(count(tracks_list.len(), "notify.next_one", "notify.next_n")));
                true
            }),
            CollectionMenuMsg::Enqueue => self.collection_with_tracks(m.source, |me, tracks_list| {
                me.enqueue_views(&tracks_list);
                me.toast(ToastKind::Success, t("notify.queued"), Some(count(tracks_list.len(), "notify.queued_one", "notify.queued_n")));
                true
            }),
            CollectionMenuMsg::AddTo(id) => {
                let name = self.data.playlists.iter().find(|p| p.id == id).map(|p| p.name.clone()).unwrap_or_default();
                self.collection_with_tracks(m.source, move |me, tracks_list| {
                    let app = me.backend.app.clone();
                    me.backend.call(
                        async move {
                            let mut added_items = 0;
                            for x in tracks_list {
                                if playlist_command::add_track_to_playlist(app.state(), id, Some(x.id), Some(x.path)).await.is_ok() {
                                    added_items += 1;
                                }
                            }
                            added_items
                        },
                        move |me, added_items| {
                            me.toast(ToastKind::Success, t("notify.added"), Some(count(added_items, "notify.added_to_one", "notify.added_to_n").replace("{name}", &name)));
                            me.load_playlists();
                            true
                        },
                    );
                    false
                });
            }
            CollectionMenuMsg::Pin => {
                let (CollectionSource::Album { id, .. }, Some(library)) = (m.source, self.data.library_id) else { return true };
                let pin = m.pinned.unwrap_or(false);
                let title = m.title;
                let app = self.backend.app.clone();
                self.backend.call(
                    async move {
                        if pin {
                            pin_command::unpin_album(app.state(), library, id).await
                        } else {
                            pin_command::pin_album(app.state(), library, id).await
                        }
                    },
                    move |me, r| {
                        match r {
                            Ok(_) => {
                                me.toast(ToastKind::Success, title, Some(t(if pin { "sidebar.unpinned" } else { "sidebar.pinned" })));
                                // La section des épinglés de la barre suit.
                                me.sidebar_load_pins();
                            }
                            Err(e) => me.toast(ToastKind::Error, t("notify.error"), Some(e)),
                        }
                        true
                    },
                );
            }
            CollectionMenuMsg::FetchCover => {
                let CollectionSource::Album { id, artist } = m.source else { return true };
                let app = self.backend.app.clone();
                let title = m.title;
                self.backend.call(async move { library_command::fetch_album_cover(app.state(), id, title, artist).await }, |me, r| {
                    match r {
                        Ok(Some(_)) => {
                            me.toast(ToastKind::Success, t("tags.cover"), Some(t("notify.cover_fetched")));
                            me.load_library_data();
                        }
                        Ok(None) => me.toast(ToastKind::Error, t("tags.cover"), Some(t("notify.cover_not_found"))),
                        Err(e) => me.toast(ToastKind::Error, t("notify.error"), Some(e)),
                    }
                    true
                });
            }
            CollectionMenuMsg::SearchCover => {
                let CollectionSource::Album { id, artist } = m.source else { return true };
                let query = match artist {
                    Some(a) if !a.is_empty() => format!("{a} {}", m.title),
                    _ => m.title,
                };
                return self.cover_search_update(crate::models::albums::CoverMsg::Open(id, query));
            }
            CollectionMenuMsg::EditTags => {
                if let CollectionSource::Album { id, .. } = m.source {
                    return self.handle(Msg::Go(Route::Tags(Some(id))));
                }
            }
            CollectionMenuMsg::Close | CollectionMenuMsg::SubPlaylists | CollectionMenuMsg::SubCover => {}
        }
        true
    }
}
