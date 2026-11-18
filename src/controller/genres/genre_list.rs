//! La liste des genres : chargement des données et réponse aux messages.

use crate::models::genres::*;
use crate::app::music_app::MusicApp;
use crate::models::app::Msg;
use crate::models::app::Route;
use crate::models::app::Section;
use crate::mapper::albums::NO_GENRE;
use crate::mapper::albums::sort_letter;
use crate::models::shared::ListWindow;
use crate::commands::library::library_command;

impl MusicApp {
    pub(crate) fn genres_open(&mut self) {
        let Some(library) = self.data.library_id else { return };
        if !self.views.genres.as_ref().is_some_and(|p| p.library_id == library) {
            self.views.genres = Some(GenresPage {
                library_id: library,
                sort: "tracks".into(),
                search: String::new(),
                main_items: false,
                without_genre: false,
                window: ListWindow::default(),
            });
        }
    }

    pub(crate) fn genres_search(&mut self, value: String) -> bool {
        let Some(page) = self.views.genres.as_mut() else { return false };
        page.search = value;
        page.window.reset();
        true
    }


    pub(crate) fn genres_update(&mut self, msg: GenresMsg) -> bool {
        match msg {
            GenresMsg::Play(name, shuffle) => {
                self.genre_play(name, shuffle);
                return true;
            }
            GenresMsg::OpenUntagged => {
                self.albums_open();
                if let Some(a) = self.views.albums.as_mut() {
                    a.genre = Some(NO_GENRE.into());
                    a.window.reset();
                }
                return self.handle(Msg::Go(Route::Library(Section::Albums)));
            }
            GenresMsg::Scrolled(offset, max) => {
                let (len, export) = (self.genres_visible().len(), self.export);
                let Some(page) = self.views.genres.as_mut() else { return false };
                return page.window.scrolled(offset, max, len, export);
            }
            GenresMsg::Letter(l) => {
                let Some(i) = self.genres_visible().iter().position(|g| sort_letter(&g.name) == l) else { return false };
                let Some(page) = self.views.genres.as_mut() else { return false };
                page.window.jump(i);
                return true;
            }
            _ => {}
        }
        let Some(page) = self.views.genres.as_mut() else { return false };
        match msg {
            GenresMsg::Sort(k) => {
                page.sort = k;
                self.menu = None;
            }
            GenresMsg::Main => {
                page.main_items = !page.main_items;
                page.without_genre = false;
            }
            GenresMsg::Untagged => {
                page.without_genre = !page.without_genre;
                page.main_items = false;
            }
            GenresMsg::ClearFilters => {
                page.search.clear();
                page.main_items = false;
                page.without_genre = false;
            }
            GenresMsg::Play(..) | GenresMsg::OpenUntagged | GenresMsg::Scrolled(..) | GenresMsg::Letter(_) => {}
        }
        page.window.reset();
        true
    }

    /// Les titres du genre, dans l'ordre ou mélangés.
    pub(crate) fn genre_play(&mut self, genre: String, shuffle: bool) {
        let Some(library) = self.data.library_id else { return };
        let app = self.backend.app.clone();
        self.backend.call(
            async move { library_command::get_tracks_by_genre(app.state(), library, genre).await.unwrap_or_default() },
            move |me, tracks| {
                let tracks = if shuffle { crate::helper::random::random_sample(&tracks, tracks.len()) } else { tracks };
                if let Some(first) = tracks.first().map(|t| t.path.clone()) {
                    me.play_tracks_view(tracks, &first);
                }
                true
            },
        );
    }
}
