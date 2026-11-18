//! La page des artistes : ouverture, recherche, défilement et réponse aux messages.

use crate::models::artists::*;
use std::time::Duration;
use crate::app::music_app::MusicApp;
use crate::models::app::Msg;
use crate::commands::library::composer_command;
use crate::models::shared::ListWindow;

impl MusicApp {
    pub(crate) fn artists_open(&mut self) {
        let Some(library) = self.data.library_id else { return };
        if self.views.artists.as_ref().is_some_and(|p| p.library_id == library) {
            return;
        }
        self.views.artists = Some(ArtistsPage {
            library_id: library,
            sort: "name".into(),
            search: String::new(),
            search_term: String::new(),
            search_gen: 0,
            without_portrait: false,
            window: ListWindow::default(),
            composers: Vec::new(),
        });
        let remote = self.data.library().is_some_and(|l| l.server_id.is_some());
        if remote {
            return;
        }
        let app = self.backend.app.clone();
        self.backend.call(
            async move { composer_command::get_composers(app.state(), library).await.unwrap_or_default() },
            move |me, composers| {
                let Some(page) = me.views.artists.as_mut().filter(|p| p.library_id == library) else { return false };
                page.composers = composers;
                true
            },
        );
    }

    pub(crate) fn artists_search(&mut self, value: String) -> bool {
        let Some(page) = self.views.artists.as_mut() else { return false };
        page.search = value;
        page.search_gen += 1;
        let n = page.search_gen;
        self.backend.call(async { tokio::time::sleep(Duration::from_millis(200)).await }, move |me, ()| {
            me.handle(Msg::Artists(ArtistsMsg::SearchSettled(n)))
        });
        true
    }


    pub(crate) fn artists_update(&mut self, msg: ArtistsMsg) -> bool {
        let export = self.export;
        if let ArtistsMsg::Mode(mode) = msg {
            if self.data.setting("artists_tab") != Some(mode) {
                if let Some(page) = self.views.artists.as_mut() {
                    page.search.clear();
                    page.search_term.clear();
                    page.without_portrait = false;
                    page.window.reset();
                }
                self.set_setting("artists_tab", mode);
            }
            return true;
        }
        if let ArtistsMsg::Scrolled(offset, max) = msg {
            let len = self.artists_filtered().len();
            let Some(page) = self.views.artists.as_mut() else { return false };
            return page.window.scrolled(offset, max, len, export);
        }
        let letter_index = match &msg {
            ArtistsMsg::Letter(l) => self.artists_filtered().iter().position(|e| e.letter == l),
            _ => None,
        };
        let Some(page) = self.views.artists.as_mut() else { return false };
        let mut reset = true;
        match msg {
            ArtistsMsg::Sort(k) => {
                page.sort = k;
                self.menu = None;
            }
            ArtistsMsg::NoPortrait => page.without_portrait = !page.without_portrait,
            ArtistsMsg::ClearFilters => {
                page.search.clear();
                page.search_term.clear();
                page.without_portrait = false;
            }
            ArtistsMsg::SearchSettled(n) => {
                if n != page.search_gen {
                    return false;
                }
                page.search_term = page.search.clone();
            }
            ArtistsMsg::Letter(_) => {
                let Some(i) = letter_index else { return false };
                page.window.jump(i);
                reset = false;
            }
            ArtistsMsg::Scrolled(..) => unreachable!("traité plus haut"),
            ArtistsMsg::Mode(_) => {}
        }
        if reset {
            page.window.reset();
        }
        true
    }
}
