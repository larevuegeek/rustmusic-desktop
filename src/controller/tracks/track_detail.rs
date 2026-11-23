//! La fiche d'un morceau : chargement des données et réponse aux messages.

use crate::models::tracks::*;
use crate::app::music_app::MusicApp;
use karbon_ui::intl::t;
use crate::models::app::Msg;
use crate::mapper::albums::album_detail::disc_order;
use crate::models::layout::ToastKind;
use crate::commands::library::library_command;

impl MusicApp {
    pub(crate) fn track_open(&mut self, id: String) {
        // Même album : ses titres restent affichés pendant le chargement.
        let guard = self.views.track.take().filter(|p| p.track.is_some());
        let album = guard.as_ref().map(|p| p.album.clone()).unwrap_or_default();
        let album_id = guard.as_ref().and_then(|p| p.track.as_ref()).and_then(|t| t.album_id.clone());
        self.views.track = Some(TrackPage { id: id.clone(), loading_state: true, track: None, credits: Vec::new(), album });
        let app = self.backend.app.clone();
        let library = self.data.library_id.unwrap_or(0);
        self.backend.call(
            async move {
                let m = library_command::get_track(app.state(), id.clone()).await.ok();
                let c = library_command::get_track_artists(app.state(), id).await.unwrap_or_default();
                let tracks_list = match m.as_ref().and_then(|m| m.album_id.clone()) {
                    Some(a) => library_command::get_tracks_by_album(app.state(), library, a).await.unwrap_or_default(),
                    None => Vec::new(),
                };
                (m, c, tracks_list)
            },
            move |me, (m, c, mut tracks_list)| {
                let Some(page) = me.views.track.as_mut() else { return false };
                tracks_list.sort_by(disc_order);
                page.loading_state = false;
                if m.as_ref().and_then(|m| m.album_id.clone()) != album_id || !tracks_list.is_empty() {
                    page.album = tracks_list;
                }
                page.track = m;
                page.credits = c;
                true
            },
        );
    }

    pub(crate) fn track_update(&mut self, msg: TrackMsg) -> bool {
        let Some(page) = self.views.track.as_ref() else { return false };
        let Some(track) = page.track.as_ref() else { return false };
        let path = track.path.clone();
        let title = track.title.clone();
        match msg {
            TrackMsg::Play => {
                let profile = self.data.profile_id().unwrap_or(0);
                let list = crate::controller::tracks::track_rows::to_queue(profile, &page.album);
                if list.is_empty() {
                    return self.handle(Msg::PlayPath(path));
                }
                self.play_list(list, &path);
            }
            TrackMsg::PlayNext => {
                let Some(x) = self.track_current().cloned() else { return false };
                let profile = self.data.profile_id().unwrap_or(0);
                if let Some(q) = crate::controller::tracks::track_rows::to_queue(profile, std::slice::from_ref(&x)).into_iter().next() {
                    self.queue_add_next(q);
                }
                self.toast(ToastKind::Success, t("track_view.queued_next"), Some(title));
            }
            TrackMsg::Like => return self.handle(Msg::Like(path)),
            TrackMsg::Reveal => return self.handle(Msg::Reveal(path)),
            TrackMsg::Menu => {
                let at = TrackMenuAt::Named(DETAIL_MORE.into());
                if let Some(x) = self.track_current().cloned() {
                    return self.track_menu_open(at, x, false);
                }
                // Hors d'un album : la ligne du morceau, lue par son chemin.
                let app = self.backend.app.clone();
                self.backend.call(
                    async move { crate::commands::playlist::playlist_view_command::get_tracks_view_by_paths(app.state(), vec![path]).await.unwrap_or_default() },
                    move |me, rows| match rows.into_iter().next() {
                        Some(x) => me.track_menu_open(at, x, false),
                        None => false,
                    },
                );
            }
            TrackMsg::Rate(stars) => {
                let rating = if track.rating == Some(stars) { None } else { Some(stars) };
                let id = track.id.clone();
                if let Some(p) = self.views.track.as_mut() {
                    if let Some(t) = p.track.as_mut() {
                        t.rating = rating;
                    }
                    for x in p.album.iter_mut().filter(|x| x.id == id) {
                        x.rating = rating;
                    }
                }
                let app = self.backend.app.clone();
                self.backend.fire(async move {
                    let _ = library_command::set_track_rating(app.state(), id, rating).await;
                });
            }
        }
        true
    }

    /// `queueState.addTrack` : juste après le morceau en cours.
    pub(crate) fn queue_add_next(&mut self, mut q: crate::entity::queue::queue_track::QueueTrack) {
        q.queue_id = uuid::Uuid::new_v4().to_string();
        let file = &mut self.data.queue.tracks;
        let current_item = self.data.player.path.as_deref().and_then(|p| file.iter().position(|x| x.path == p));
        let rank = current_item.map_or(file.len(), |i| i + 1);
        file.insert(rank, q);
        for (i, t) in file.iter_mut().enumerate() {
            t.position = i as i32;
        }
        let all = file.clone();
        let profile = self.data.profile_id().unwrap_or(0);
        let app = self.backend.app.clone();
        self.backend.fire(async move {
            let _ = crate::commands::playback::queue_command::replace_queue_tracks(app.state(), profile, all).await;
        });
    }
}
