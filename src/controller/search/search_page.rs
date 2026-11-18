//! La page de recherche : chargement des données et réponse aux messages.

use crate::models::search::*;
use crate::app::music_app::MusicApp;
use crate::models::app::Msg;
use crate::models::app::Route;
use crate::controller::tracks::track_rows::to_queue;
use crate::commands::library::search_command;
use crate::commands::playlist::playlist_view_command;
use crate::query::track::track_list_row::TrackListRow;
use crate::models::app::Menu;

impl MusicApp {
    pub(crate) fn search_open(&mut self, query: String) {
        if self.views.search.query == query && !self.views.search.results.is_empty() {
            return;
        }
        self.views.search = SearchPage { query: query.clone(), loading: true, filtre: "all", ..Default::default() };
        if query.chars().count() < 2 {
            self.views.search.loading = false;
            return;
        }
        let profile = self.data.profile_id().unwrap_or(0);
        let source = self.source_id();
        let app = self.backend.app.clone();
        let q = query.clone();
        self.backend.call(
            async move {
                let results = search_command::search(app.state(), q, Some(50), profile, source).await.unwrap_or_default();
                let paths: Vec<String> = results.iter().filter(|r| r.result_type == "track").filter_map(|r| r.path.clone()).collect();
                let records = playlist_view_command::get_tracks_view_by_paths(app.state(), paths).await.unwrap_or_default();
                (results, records)
            },
            move |me, (results, records)| {
                if me.views.search.query != query {
                    return false;
                }
                me.views.search.results = results;
                me.views.search.records = records;
                me.views.search.loading = false;
                true
            },
        );
    }

    pub(crate) fn search_update(&mut self, msg: SearchMsg) -> bool {
        match msg {
            SearchMsg::Filter(f) => self.views.search.filtre = f,
            SearchMsg::Open(n) => {
                let Some(r) = self.views.search.results.get(n).cloned() else { return false };
                match r.result_type.as_str() {
                    "track" => {
                        let Some(path) = r.path.clone() else { return false };
                        self.search_play(&path);
                    }
                    "album" => return self.handle(Msg::Go(Route::Album(r.id))),
                    "artist" => return self.handle(Msg::Go(Route::Artist(r.id))),
                    "composer" => return self.handle(Msg::Go(Route::Composer(r.title))),
                    _ => {}
                }
            }
            SearchMsg::PlayAll => {
                if let Some(path) = self.views.search.results.iter().find(|r| r.result_type == "track").and_then(|r| r.path.clone()) {
                    self.search_play(&path);
                }
            }
        }
        true
    }

    /// `handlePlayTrack(path, toQueue(tracks))` : les morceaux trouvés, dans l'ordre.
    pub(crate) fn search_play(&mut self, path: &str) {
        let profile = self.data.profile_id().unwrap_or(0);
        let page = &self.views.search;
        let records: Vec<TrackListRow> = page
            .results
            .iter()
            .filter(|r| r.result_type == "track")
            .filter_map(|r| r.path.as_ref().and_then(|p| page.records.iter().find(|f| f.path == *p)).cloned())
            .collect();
        let queue = to_queue(profile, &records);
        if !queue.is_empty() {
            self.play_list(queue, path);
        } else {
            self.handle(Msg::PlayPath(path.to_string()));
        }
    }
}

impl MusicApp {
    pub(crate) fn do_search(&mut self) {
        let query = self.search.trim().to_string();
        self.search_generation += 1;
        let generation = self.search_generation;
        self.suggestion = None;
        if query.chars().count() < 2 {
            self.data.search.clear();
            if self.open(Menu::Search) {
                self.menu = None;
            }
            return;
        }
        let Some(profile) = self.data.profile_id() else {
            return;
        };
        let source = self.source_id();
        let app = self.backend.app.clone();
        self.backend.call(
            async move { search_command::search(app.state(), query, Some(30), profile, source).await },
            move |me, results| {
                if generation != me.search_generation {
                    return false;
                }
                me.data.search = results.unwrap_or_default();
                if me.data.search.is_empty() {
                    if me.open(Menu::Search) {
                        me.menu = None;
                    }
                } else {
                    me.menu = Some(Menu::Search);
                }
                true
            },
        );
    }
}
