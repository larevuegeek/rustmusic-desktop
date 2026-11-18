//! Les années : chargement des données et réponse aux messages.

use crate::models::years::*;
use crate::mapper::years::year_list::*;
use crate::app::music_app::MusicApp;
use crate::models::app::Msg;
use crate::models::app::Route;
use crate::commands::library::library_command;

impl MusicApp {
    pub(crate) fn years_open(&mut self) {
        let Some(library) = self.data.library_id else { return };
        if !self.views.years.as_ref().is_some_and(|p| p.library_id == library) {
            self.views.years = Some(YearsPage { library_id: library, recent_items: true, ..Default::default() });
        }
    }

    pub(crate) fn years_update(&mut self, msg: YearsMsg) -> bool {
        match msg {
            YearsMsg::Mix(d, y) => {
                self.period_mix(d, y);
                return true;
            }
            YearsMsg::Open(d, y) => {
                self.period_open(d);
                if let Some(page) = self.views.genre.as_mut() {
                    page.period = Some((d, y));
                }
                return self.handle(Msg::Go(Route::Decade(d)));
            }
            _ => {}
        }
        let Some(page) = self.views.years.as_mut() else { return false };
        match msg {
            YearsMsg::Order(r) => {
                page.recent_items = r;
                page.start.clear();
                self.menu = None;
            }
            YearsMsg::Year(d, y) => {
                page.choice.insert(d, y);
                page.start.remove(&d);
            }
            YearsMsg::Jump(d) => page.since = d,
            YearsMsg::Slide(d, before) => {
                let start = page.start.entry(d).or_default();
                *start = if before { *start + PAGE } else { start.saturating_sub(PAGE) };
            }
            YearsMsg::Mix(..) | YearsMsg::Open(..) => {}
        }
        true
    }

    /// Cent titres d'une décennie entière, ou d'une seule année.
    pub(crate) fn period_mix(&mut self, decade: i64, year: Option<i64>) {
        let Some(library) = self.data.library_id else { return };
        let app = self.backend.app.clone();
        self.backend.call(
            async move {
                let (kind, d, y) = match year {
                    Some(y) => ("annee", None, Some(y)),
                    None => ("decennie", Some(decade), None),
                };
                library_command::get_mix_tracks(app.state(), library, kind.into(), None, d, y, 100).await.unwrap_or_default()
            },
            |me, tracks| {
                if let Some(first) = tracks.first().map(|t| t.path.clone()) {
                    me.play_tracks_view(tracks, &first);
                }
                true
            },
        );
    }
}
