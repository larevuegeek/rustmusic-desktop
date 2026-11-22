//! Les lignes de morceaux : chargement des données et réponse aux messages.

use crate::models::tracks::*;
use crate::app::music_app::MusicApp;
use crate::models::app::Msg;
use crate::models::app::Route;
use crate::entity::queue::queue_track::QueueTrack;
use crate::query::track::track_list_row::TrackListRow;

/// `TrackListRow` → morceau de file d'attente.
pub fn to_queue(profile: i64, tracks: &[TrackListRow]) -> Vec<QueueTrack> {
    tracks
        .iter()
        .map(|t| QueueTrack {
            queue_id: String::new(),
            profile_id: profile,
            position: 0,
            path: t.path.clone(),
            title: t.title.clone(),
            artist: t.artist.clone(),
            duration: t.duration,
            cover: t.thumbnail_path.clone(),
        })
        .collect()
}

impl MusicApp {
    pub(crate) fn row_update(&mut self, msg: RowMsg) -> bool {
        let Some(tracks) = self.table_tracks() else {
            return false;
        };
        let base = self.table_base();
        let track = |i: usize| i.checked_sub(base).and_then(|k| tracks.get(k));
        let profile = self.data.profile_id().unwrap_or(0);
        match msg {
            RowMsg::Play(i) => {
                let Some(path) = track(i).map(|t| t.path.clone()) else { return false };
                self.play_list(to_queue(profile, &tracks), &path);
            }
            RowMsg::Open(_) if self.selecting() || self.data.flag("single_click_play") => return false,
            RowMsg::Open(i) => {
                let Some(path) = track(i).map(|t| t.path.clone()) else { return false };
                self.play_list(to_queue(profile, &tracks), &path);
            }
            RowMsg::Click(i) if self.selecting() => {
                let picked = track(i).cloned();
                return self.selection_toggle_row(i, picked);
            }
            RowMsg::Click(i) => {
                let Some(path) = track(i).map(|t| t.path.clone()) else { return false };
                if self.data.flag("single_click_play") {
                    self.play_list(to_queue(profile, &tracks), &path);
                } else {
                    // Sans lecture en cours, le morceau est désigné
                    // (chargé dans le lecteur), sans démarrer.
                    self.select_track(to_queue(profile, &tracks), &path);
                }
            }
            RowMsg::Like(i) => {
                let Some(path) = track(i).map(|t| t.path.clone()) else { return false };
                return self.handle(Msg::Like(path));
            }
            RowMsg::Rate(i, stars) => {
                let Some(track) = track(i) else { return false };
                let rating = if track.rating == Some(stars) { None } else { Some(stars) };
                let id = track.id.clone();
                self.set_rating_in_views(&id, rating);
                let app = self.backend.app.clone();
                self.backend.fire(async move {
                    let _ = crate::commands::library::library_command::set_track_rating(app.state(), id, rating).await;
                });
            }
            RowMsg::Menu(i, at) => {
                let Some(track) = track(i).cloned() else { return false };
                let nav = !matches!(self.route(), Route::Track(_));
                return self.track_menu_open(at, track, nav);
            }
            RowMsg::Sort(key) => {
                return match self.route() {
                    Route::Album(_) => self.album_sort(key),
                    Route::Artist(_) => self.artist_sort(key, true),
                    Route::Genre(_) | Route::Decade(_) | Route::Composer(_) => self.genre_sort(key, true),
                    Route::Playlist(_) | Route::Liked | Route::Recent => self.playlist_sort(key),
                    _ => self.tracks_sort_column(key),
                };
            }
        }
        true
    }

    /// La note change aussi dans les listes déjà chargées.
    pub(crate) fn set_rating_in_views(&mut self, id: &str, rating: Option<f64>) {
        if let Some(page) = self.views.tracks.as_mut() {
            for t in page.tracks.iter_mut().filter(|t| t.id == id) {
                t.rating = rating;
            }
        }
        if let Some(page) = self.views.album.as_mut() {
            for t in page.tracks.iter_mut().filter(|t| t.id == id) {
                t.rating = rating;
            }
        }
        if let Some(page) = self.views.artist.as_mut() {
            for t in page.tracks.iter_mut().filter(|t| t.id == id) {
                t.rating = rating;
            }
        }
        if let Some(page) = self.views.genre.as_mut() {
            for t in page.tracks.iter_mut().filter(|t| t.id == id) {
                t.rating = rating;
            }
        }
        if let Some(page) = self.views.playlist.as_mut() {
            for t in page.tracks.iter_mut().filter(|t| t.id == id) {
                t.rating = rating;
            }
        }
    }
}
