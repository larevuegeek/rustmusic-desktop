//! La fiche d'un genre : chargement des données et réponse aux messages.

use crate::models::genres::*;
use crate::app::music_app::MusicApp;
use crate::commands::library::library_command;

impl MusicApp {
    pub(crate) fn genre_open(&mut self, name: String) {
        if self.views.genre.as_ref().is_some_and(|p| p.name == name && p.period.is_none()) {
            return;
        }
        self.views.genre = Some(GenrePage::new(name, None));
        self.collection_load();
    }

    /// `composers/[composer]` : les morceaux portant ce compositeur.
    pub(crate) fn composer_open(&mut self, name: String) {
        if self.views.genre.as_ref().is_some_and(|p| p.name == name && p.composer) {
            return;
        }
        let mut page = GenrePage::new(name.clone(), None);
        page.composer = true;
        page.sort = "album".into();
        self.views.genre = Some(page);
        self.collection_load();
        // Sa photo de compositeur, si l'artiste du même nom n'en a pas.
        let artist_name = self.data.artists.iter().any(|a| a.name.trim().eq_ignore_ascii_case(name.trim()) && a.thumbnail_path.is_some());
        if !artist_name {
            let app = self.backend.app.clone();
            let n = name.clone();
            self.backend.call(
                async move { crate::commands::library::composer_command::fetch_composer_image(app.state(), n).await.ok().flatten() },
                move |me, photo| {
                    let Some(page) = me.views.genre.as_mut().filter(|p| p.composer && p.name == name) else { return false };
                    page.photo = photo;
                    true
                },
            );
        }
    }

    /// `years/[decade]` : la décennie, ou l'année choisie avant d'y venir.
    pub(crate) fn period_open(&mut self, decade: i64) {
        if !self.views.genre.as_ref().is_some_and(|p| p.period.is_some_and(|(d, _)| d == decade)) {
            self.views.genre = Some(GenrePage::new(String::new(), Some((decade, None))));
        }
        self.collection_load();
    }

    /// Les titres de la collection ouverte (genre, décennie ou année).
    pub(crate) fn collection_load(&mut self) {
        let Some(library) = self.data.library_id else { return };
        let Some(page) = self.views.genre.as_mut() else { return };
        page.loading = false;
        page.error = false;
        let (name, period, composer) = (page.name.clone(), page.period, page.composer);
        let app = self.backend.app.clone();
        let g = name.clone();
        self.backend.call(
            async move {
                if composer {
                    return crate::commands::library::composer_command::get_tracks_by_composer(app.state(), library, g).await;
                }
                match period {
                    Some((d, y)) => {
                        let (a, b) = y.map(|y| (y, y)).unwrap_or((d, d + 9));
                        library_command::get_tracks_by_years(app.state(), library, a, b).await
                    }
                    None => library_command::get_tracks_by_genre(app.state(), library, g).await,
                }
            },
            move |me, result| {
                let Some(page) = me.views.genre.as_mut().filter(|p| p.name == name && p.period == period && p.composer == composer) else { return false };
                page.loading = true;
                match result {
                    Ok(tracks) => page.tracks = tracks,
                    Err(_) => page.error = true,
                }
                true
            },
        );
    }

    /// La même clé inverse le sens ; année et ajout partent des plus récents.
    pub(crate) fn genre_sort(&mut self, key: String, from_table: bool) -> bool {
        let Some(page) = self.views.genre.as_mut() else { return false };
        if page.sort == key {
            page.asc = !page.asc;
        } else {
            page.asc = from_table || !(key == "year" || key == "created_at");
            page.sort = key;
        }
        page.window.reset();
        true
    }


    pub(crate) fn genre_update(&mut self, msg: GenreMsg) -> bool {
        match msg {
            GenreMsg::Sort(key) => return self.genre_sort(key, false),
            GenreMsg::PlayAll | GenreMsg::Shuffle => {
                let tracks = if msg == GenreMsg::Shuffle {
                    let l = self.views.genre.as_ref().map(|p| p.tracks.clone()).unwrap_or_default();
                    crate::helper::random::random_sample(&l, l.len())
                } else {
                    self.genre_sorted()
                };
                if let Some(first) = tracks.first().map(|t| t.path.clone()) {
                    self.play_tracks_view(tracks, &first);
                }
                return true;
            }
            GenreMsg::Year(y) => {
                let Some(page) = self.views.genre.as_mut() else { return false };
                if let Some((_, year)) = page.period.as_mut() {
                    *year = y;
                    page.window.reset();
                    self.collection_load();
                }
                return true;
            }
            GenreMsg::Scrolled(offset, max) => {
                let (len, export) = (self.genre_list_len(), self.export);
                let Some(page) = self.views.genre.as_mut() else { return false };
                return page.window.scrolled(offset, max, len, export);
            }
            _ => {}
        }
        let Some(page) = self.views.genre.as_mut() else { return false };
        match msg {
            GenreMsg::View(v) => {
                page.view = Some(v);
                page.window.reset();
            }
            GenreMsg::SortAlbums(k) => {
                page.sort_albums = k;
                page.window.reset();
            }
            _ => {}
        }
        true
    }
}
