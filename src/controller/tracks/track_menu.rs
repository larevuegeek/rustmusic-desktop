use crate::models::tracks::*;
use crate::app::music_app::MusicApp;
use karbon_ui::intl::t;
use crate::models::app::Msg;
use crate::models::app::Route;
use crate::models::layout::ToastKind;
use crate::commands::playlist::playlist_command;
use crate::query::track::track_list_row::TrackListRow;

impl MusicApp {
    /// Ouvre le menu d'un morceau.
    pub(crate) fn track_menu_open(&mut self, at: TrackMenuAt, track: TrackListRow, nav: bool) -> bool {
        self.menu = None;
        self.views.track_menu = Some(TrackMenu { at, track, sub: false, nav });
        true
    }

    pub(crate) fn track_menu_update(&mut self, msg: TrackMenuMsg) -> bool {
        let Some(m) = self.views.track_menu.as_mut() else { return false };
        if msg == TrackMenuMsg::Sub {
            m.sub = !m.sub;
            return true;
        }
        let Some(m) = self.views.track_menu.take() else { return false };
        let x = m.track;
        match msg {
            TrackMenuMsg::Play => return self.handle(Msg::PlayPath(x.path)),
            TrackMenuMsg::PlayNext => {
                let profile = self.data.profile_id().unwrap_or(0);
                if let Some(q) = crate::controller::tracks::track_rows::to_queue(profile, std::slice::from_ref(&x)).into_iter().next() {
                    self.queue_add_next(q);
                }
                self.toast(ToastKind::Success, t("notify.added_next"), Some(t("notify.track_next")));
            }
            TrackMenuMsg::Enqueue => {
                self.enqueue_views(std::slice::from_ref(&x));
                self.toast(ToastKind::Success, t("notify.queued"), Some(t("notify.track_queued")));
            }
            TrackMenuMsg::Like => return self.handle(Msg::Like(x.path)),
            TrackMenuMsg::AddTo(id) => {
                let name = self.data.playlists.iter().find(|p| p.id == id).map(|p| p.name.clone()).unwrap_or_default();
                let app = self.backend.app.clone();
                let (track_id, path) = (Some(x.id.clone()), Some(x.path.clone()));
                self.backend.call(async move { playlist_command::add_track_to_playlist(app.state(), id, track_id, path).await }, move |me, r| {
                    match r {
                        Ok(()) => {
                            me.toast(ToastKind::Success, t("notify.added"), Some(t("notify.added_to").replace("{name}", &name)));
                            me.load_playlists();
                        }
                        Err(e) => {
                            let message = if e == "server_playlist_foreign_track" { t("servers.playlist_foreign") } else { e };
                            me.toast(ToastKind::Error, t("notify.error"), Some(message));
                        }
                    }
                    true
                });
            }
            TrackMenuMsg::Track => return self.handle(Msg::Go(Route::Track(x.id))),
            TrackMenuMsg::Album => {
                if let Some(a) = x.album_id {
                    return self.handle(Msg::Go(Route::Album(a)));
                }
            }
            TrackMenuMsg::Artist => {
                if let Some(a) = x.artist_id {
                    return self.handle(Msg::Go(Route::Artist(a)));
                }
            }
            TrackMenuMsg::NewPlaylist => return self.playlist_form_update(crate::models::playlists::PlaylistFormMsg::New(Some((x.id, x.path)))),
            TrackMenuMsg::Tags => return self.tag_editor_update(crate::models::tags::TagEditorMsg::Open(x.path)),
            TrackMenuMsg::Close | TrackMenuMsg::Sub => {}
        }
        true
    }
}
