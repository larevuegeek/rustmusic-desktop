//! La page générique : liste de morceaux ou grille de cartes.

use crate::models::tracks::*;
use std::rc::Rc;
use crate::helper::string::string::display_title;
use crate::models::app::Route;
use crate::mapper::home::readable_artists;
use crate::entity::queue::queue_track::QueueTrack;
use crate::query::track::track_list_row::TrackListRow;
use crate::app::music_app::MusicApp;

/// `TrackListRow` → morceau de file et nom d'album.
pub(crate) fn from_views(profile: i64, tracks: Vec<TrackListRow>) -> (Vec<QueueTrack>, Vec<String>) {
    tracks
        .into_iter()
        .map(|t| {
            let album = t.album.clone().unwrap_or_default();
            (
                QueueTrack {
                    queue_id: String::new(),
                    profile_id: profile,
                    position: 0,
                    title: display_title(Some(&t.title), &t.path, &karbon_ui::intl::t("common.unknown_title")),
                    artist: readable_artists(t.artist.as_deref()),
                    duration: t.duration,
                    cover: t.thumbnail_path,
                    path: t.path,
                },
                album,
            )
        })
        .unzip()
}

impl MusicApp {
    /// La page de la route courante, une fois demandée.
    pub fn current_page(&self) -> Option<&Page> {
        self.data.page.as_ref().filter(|p| p.route == *self.route())
    }

    /// `true` si la page de la route courante est chargée.
    pub fn page_ready(&self) -> bool {
        self.current_page().is_some_and(|p| p.kind != PageKind::Loading)
    }
}

impl Page {
    pub fn new(route: Route, kind: PageKind) -> Self {
        Self {
            title: route.title(),
            route,
            kind,
            kicker: String::new(),
            subtitle: String::new(),
            cover: None,
            rows: Rc::new(Vec::new()),
            queue: Vec::new(),
        }
    }

    /// Les pochettes que la page montre (en-tête).
    pub fn covers(&self) -> Vec<&str> {
        self.cover.as_deref().into_iter().collect()
    }
}
