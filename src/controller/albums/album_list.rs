//! La page des albums : ouverture, recherche, filtres, défilement et réponse aux messages.

use crate::models::albums::*;
use crate::models::app::Msg;
use std::time::Duration;
use crate::app::music_app::MusicApp;
use crate::models::app::Menu;

impl MusicApp {
    pub(crate) fn albums_open(&mut self) {
        let Some(library) = self.data.library_id else { return };
        if !self.views.albums.as_ref().is_some_and(|p| p.library_id == library) {
            self.views.albums = Some(AlbumsPage::new(library));
        }
    }

    pub(crate) fn albums_search(&mut self, value: String) -> bool {
        let Some(page) = self.views.albums.as_mut() else { return false };
        page.search = value;
        page.search_gen += 1;
        let n = page.search_gen;
        self.backend.call(async { tokio::time::sleep(Duration::from_millis(200)).await }, move |me, ()| {
            me.handle(Msg::Albums(AlbumsMsg::SearchSettled(n)))
        });
        true
    }


    pub(crate) fn albums_update(&mut self, msg: AlbumsMsg) -> bool {
        let export = self.export;
        let popup = |me: &mut MusicApp, id: &'static str| {
            let open = me.popup(id);
            me.menu = if open { None } else { Some(Menu::Popup(id)) };
        };
        match msg {
            AlbumsMsg::GenreMenu => {
                popup(self, "albums.genre");
                return true;
            }
            AlbumsMsg::YearMenu => {
                popup(self, "albums.year");
                return true;
            }
            AlbumsMsg::View(mode) => {
                self.views.view_mode = mode;
                return true;
            }
            AlbumsMsg::Play(id) => {
                self.play_album(id);
                return true;
            }
            AlbumsMsg::Enqueue(id) => {
                self.enqueue_album(id);
                return true;
            }
            _ => {}
        }
        if let AlbumsMsg::Scrolled(offset, max) = msg {
            let len = self.albums_filtered().len();
            let Some(page) = self.views.albums.as_mut() else { return false };
            return page.window.scrolled(offset, max, len, export);
        }
        let letter_index = match &msg {
            AlbumsMsg::Letter(l) => self.albums_filtered().iter().position(|(_, x)| x == l),
            _ => None,
        };
        let Some(page) = self.views.albums.as_mut() else { return false };
        let mut reset = true;
        match msg {
            AlbumsMsg::Sort(k) => {
                page.sort = k;
                self.menu = None;
            }
            AlbumsMsg::Hires => page.hires = !page.hires,
            AlbumsMsg::Compilations => page.compilations = !page.compilations,
            AlbumsMsg::NoCover => page.without_cover = !page.without_cover,
            AlbumsMsg::Genre(g) => {
                page.genre = g;
                self.menu = None;
            }
            AlbumsMsg::Year(y) => {
                page.year = y;
                self.menu = None;
            }
            AlbumsMsg::ClearFilters => {
                page.search.clear();
                page.search_term.clear();
                page.hires = false;
                page.compilations = false;
                page.without_cover = false;
                page.genre = None;
                page.year = None;
            }
            AlbumsMsg::SearchSettled(n) => {
                if n != page.search_gen {
                    return false;
                }
                page.search_term = page.search.clone();
            }
            AlbumsMsg::Letter(_) => {
                let Some(i) = letter_index else { return false };
                page.window.jump(i);
                reset = false;
            }
            AlbumsMsg::Scrolled(..) => unreachable!("traité plus haut"),
            _ => {}
        }
        if reset {
            page.window.reset();
        }
        true
    }

    /// Les morceaux de l'album à la fin de la file.
    pub(crate) fn enqueue_album(&mut self, album_id: String) {
        let Some(library) = self.data.library_id else { return };
        let app = self.backend.app.clone();
        self.backend.call(
            async move {
                crate::commands::library::library_command::get_tracks_by_album(app.state(), library, album_id)
                    .await
                    .unwrap_or_default()
            },
            move |me, tracks| {
                me.enqueue_views(&tracks);
                true
            },
        );
    }

    /// `queueState.enqueue` : ces morceaux à la fin de la file.
    pub(crate) fn enqueue_views(&mut self, tracks: &[crate::query::track::track_list_row::TrackListRow]) {
        let profile = self.data.profile_id().unwrap_or(0);
        let mut added = crate::controller::tracks::track_rows::to_queue(profile, tracks);
        for t in added.iter_mut() {
            t.queue_id = uuid::Uuid::new_v4().to_string();
        }
        self.data.queue.tracks.extend(added);
        for (i, t) in self.data.queue.tracks.iter_mut().enumerate() {
            t.position = i as i32;
        }
        let all = self.data.queue.tracks.clone();
        let app = self.backend.app.clone();
        self.backend.fire(async move {
            let _ = crate::commands::playback::queue_command::replace_queue_tracks(app.state(), profile, all).await;
        });
    }
}
