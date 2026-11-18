//! La fiche d'une playlist : chargement des données et réponse aux messages.

use crate::models::playlists::*;
use crate::app::music_app::MusicApp;
use karbon_ui::intl::t;
use crate::models::app::Route;
use crate::commands::playback::recent_command;
use crate::commands::playlist::{playlist_command, playlist_view_command};
use crate::query::track::track_list_row::TrackListRow;

/// Un morceau aimé ou écouté hors bibliothèque reste listé.
pub(crate) fn outside_library(path: &str, title: Option<&str>, artist: Option<&str>, album: Option<&str>, duration: Option<f64>, cover: Option<&str>) -> Option<TrackListRow> {
    let filename = path.rsplit(['\\', '/']).next().unwrap_or(path).to_string();
    let title = title.map(String::from).unwrap_or_else(|| filename.rsplit_once('.').map(|(b, _)| b.to_string()).unwrap_or(filename.clone()));
    serde_json::from_value(serde_json::json!({
        "id": format!("hors:{path}"),
        "path": path,
        "filename": filename,
        "extension": path.rsplit_once('.').map(|(_, e)| e).unwrap_or(""),
        "size": 0,
        "status": "",
        "is_available": true,
        "title": title,
        "title_normalized": title.to_lowercase(),
        "artist": artist,
        "album": album,
        "disc_number": 1,
        "duration": duration,
        "thumbnail_path": cover,
        "play_count": 0,
        "favorite": false,
        "created_at": "1970-01-01T00:00:00Z",
        "updated_at": "1970-01-01T00:00:00Z",
    }))
    .ok()
}

impl MusicApp {
    pub(crate) fn playlist_page(&mut self, route: Route, title: String, kicker: String, color: &str, icon: &str) {
        let keep = self.views.playlist.as_ref().is_some_and(|p| p.route == route);
        if !keep {
            self.views.playlist = Some(PlaylistPage {
                route,
                title,
                kicker,
                color: color.into(),
                icon: icon.into(),
                smart: false,
                server: false,
                tracks: Vec::new(),
                items: Vec::new(),
                loading: true,
                failed: false,
                sort: "order".into(),
                asc: true,
                confirm: false,
            });
        }
    }

    pub(crate) fn playlist_open(&mut self, id: i64) {
        let (title, color, icon, smart) = self
            .data
            .playlists
            .iter()
            .find(|p| p.id == id)
            .map(|p| (p.name.clone(), p.color.clone(), p.icon.clone(), p.is_smart))
            .unwrap_or_default();
        self.playlist_page(Route::Playlist(id), title, t(if smart { "playlist.new_smart" } else { "home.playlist" }), &color, &icon);
        let app = self.backend.app.clone();
        self.backend.call(
            async move {
                let p = playlist_command::get_playlist(app.state(), id).await;
                let view = playlist_view_command::get_playlist_tracks_view(app.state(), id).await.unwrap_or_default();
                let items = playlist_command::get_playlist_tracks(app.state(), id).await.unwrap_or_default();
                (p, view, items)
            },
            move |me, (p, view, items)| {
                let Some(page) = me.views.playlist.as_mut().filter(|x| x.route == Route::Playlist(id)) else { return false };
                page.loading = false;
                match p {
                    Ok(p) => {
                        page.title = p.name;
                        page.color = p.color;
                        page.icon = p.icon;
                        page.smart = p.is_smart;
                        page.server = p.server_id.is_some();
                        page.kicker = t(if p.is_smart { "playlist.new_smart" } else { "home.playlist" });
                        page.tracks = view;
                        page.items = items.into_iter().map(|x| (x.library_track_id, x.playlist_item_id)).collect();
                    }
                    Err(_) => page.failed = true,
                }
                true
            },
        );
    }

    pub(crate) fn liked_open(&mut self) {
        self.playlist_page(Route::Liked, t("playlist_page.liked_title"), t("playlist_page.collection"), "#f43f5e", "material-symbols:favorite-rounded");
        let Some(profile) = self.data.profile_id() else { return };
        let source = self.source_id();
        let app = self.backend.app.clone();
        self.backend.call(
            async move {
                let likes = playlist_command::get_tracks_liked(app.state(), profile, source).await.unwrap_or_default();
                let mut paths: Vec<String> = Vec::new();
                for l in &likes {
                    if !l.path.is_empty() && !paths.contains(&l.path) {
                        paths.push(l.path.clone());
                    }
                }
                let view = playlist_view_command::get_tracks_view_by_paths(app.state(), paths.clone()).await.unwrap_or_default();
                // Dans l'ordre des likes ; hors bibliothèque, le titre aimé reste là.
                paths
                    .iter()
                    .filter_map(|p| {
                        view.iter().find(|x| x.path == *p).cloned().or_else(|| {
                            let l = likes.iter().find(|l| l.path == *p)?;
                            outside_library(p, l.title.as_deref(), l.artist.as_deref(), l.album.as_deref(), l.duration, l.thumbnail_path.as_deref())
                        })
                    })
                    .collect::<Vec<_>>()
            },
            |me, tracks| {
                let Some(page) = me.views.playlist.as_mut().filter(|x| x.route == Route::Liked) else { return false };
                page.loading = false;
                page.tracks = tracks;
                true
            },
        );
    }

    pub(crate) fn recent_open(&mut self) {
        self.playlist_page(Route::Recent, t("playlist_page.recent_title"), t("playlist_page.history"), "#0ea5e9", "material-symbols:history-rounded");
        let recent = self.data.recent.clone();
        let app = self.backend.app.clone();
        self.backend.call(
            async move {
                let paths: Vec<String> = recent.iter().map(|r| r.path.clone()).filter(|p| !p.is_empty()).collect();
                let view = playlist_view_command::get_tracks_view_by_paths(app.state(), paths.clone()).await.unwrap_or_default();
                paths
                    .iter()
                    .filter_map(|p| {
                        view.iter().find(|x| x.path == *p).cloned().or_else(|| {
                            let r = recent.iter().find(|r| r.path == *p)?;
                            outside_library(p, r.title.as_deref(), r.artist.as_deref(), r.album.as_deref(), r.duration, r.thumbnail_path.as_deref())
                        })
                    })
                    .collect::<Vec<_>>()
            },
            |me, tracks| {
                let Some(page) = me.views.playlist.as_mut().filter(|x| x.route == Route::Recent) else { return false };
                page.loading = false;
                page.tracks = tracks;
                true
            },
        );
    }

    pub(crate) fn playlist_sort(&mut self, key: String) -> bool {
        let Some(page) = self.views.playlist.as_mut() else { return false };
        if page.sort == key {
            page.asc = !page.asc;
        } else {
            page.sort = key;
            page.asc = true;
        }
        true
    }

    pub(crate) fn playlist_update(&mut self, msg: PlaylistMsg) -> bool {
        match msg {
            PlaylistMsg::Sort(k) => return self.playlist_sort(k),
            PlaylistMsg::PlayAll | PlaylistMsg::Shuffle => {
                let tracks = if msg == PlaylistMsg::Shuffle {
                    let l = self.views.playlist.as_ref().map(|p| p.tracks.clone()).unwrap_or_default();
                    crate::helper::random::random_sample(&l, l.len())
                } else {
                    self.playlist_sorted()
                };
                if let Some(first) = tracks.first().map(|t| t.path.clone()) {
                    self.play_tracks_view(tracks, &first);
                }
            }
            PlaylistMsg::Pin => {
                let Some(Route::Playlist(id)) = self.views.playlist.as_ref().map(|p| p.route.clone()) else { return false };
                let pinned = self.data.playlists.iter().find(|p| p.id == id).is_some_and(|p| p.pinned);
                return self.playlists_update(crate::models::playlists::PlaylistsMsg::Pin(id, !pinned));
            }
            PlaylistMsg::AskDelete(open) => {
                if let Some(page) = self.views.playlist.as_mut() {
                    page.confirm = open;
                }
            }
            PlaylistMsg::Delete => {
                let Some(Route::Playlist(id)) = self.views.playlist.as_ref().map(|p| p.route.clone()) else { return false };
                let app = self.backend.app.clone();
                self.backend.call(
                    async move {
                        let _ = playlist_command::delete_playlist(app.state(), id).await;
                    },
                    |me, ()| {
                        me.load_playlists();
                        true
                    },
                );
                self.views.playlist = None;
                return self.handle(crate::models::app::Msg::Go(Route::Playlists));
            }
            PlaylistMsg::ClearHistory => {
                let Some(profile) = self.data.profile_id() else { return false };
                let source = self.source_id();
                self.data.recent.clear();
                if let Some(page) = self.views.playlist.as_mut() {
                    page.tracks.clear();
                }
                let app = self.backend.app.clone();
                self.backend.fire(async move {
                    let _ = recent_command::clear_recent_files(app.state(), profile, source).await;
                });
            }
            PlaylistMsg::Remove(i) => {
                let sorted = self.playlist_sorted();
                let Some(track) = sorted.get(i).cloned() else { return false };
                let Some(page) = self.views.playlist.as_mut() else { return false };
                page.tracks.retain(|x| x.id != track.id);
                match page.route {
                    Route::Liked => return self.handle(crate::models::app::Msg::Like(track.path)),
                    Route::Playlist(_) => {
                        let Some(item) = page.items.iter().find(|(id, _)| *id == track.id).map(|(_, item)| *item) else { return true };
                        let app = self.backend.app.clone();
                        self.backend.call(
                            async move {
                                let _ = playlist_command::remove_track_from_playlist(app.state(), item).await;
                            },
                            |me, ()| {
                                me.load_playlists();
                                true
                            },
                        );
                    }
                    _ => {}
                }
            }
        }
        true
    }
}
