use crate::models::tracks::*;
use karbon_ui::intl;
use crate::app::music_app::MusicApp;
use karbon_ui::intl::t;
use crate::models::app::Route;
use crate::models::app::Section;
use crate::commands::library::library_command;
use crate::controller::tracks::track_rows::to_queue;
use crate::models::layout::ToastKind;
use crate::commands::playlist::playlist_command;
use crate::query::track::track_list_row::TrackListRow;

/// « 1 morceau sélectionné » / « n morceaux sélectionnés ».
pub fn count_label(n: usize, one: &str, many: &str) -> String {
    t(if n == 1 { one } else { many }).replace("{n}", &intl::number(n as i64))
}

impl MusicApp {
    /// Un clic de ligne en mode sélection : coche ou décoche.
    pub(crate) fn selection_toggle_row(&mut self, i: usize, track: Option<TrackListRow>) -> bool {
        let Some(s) = self.views.selection.as_mut() else { return false };
        s.all = false;
        if s.picked.remove(&i).is_none() {
            if let Some(tr) = track {
                s.picked.insert(i, tr);
            }
        }
        true
    }

    /// Coche toute la liste de la page. La page Morceaux ne tient qu'une fenêtre de lignes :
    /// toute la liste filtrée est lue d'abord.
    fn selection_select_all(&mut self) -> bool {
        if let (Route::Library(Section::Tracks), Some(page)) = (self.route(), self.views.tracks.as_ref()) {
            let p = page.params();
            let total = page.total.max(0);
            let app = self.backend.app.clone();
            self.backend.call(
                async move {
                    library_command::get_tracks_paginated(
                        app.state(), p.library, 0, total, p.sort_by, p.sort_dir, p.filter, p.missing_cover, p.quality, p.favorites, p.genre,
                    )
                    .await
                },
                |me, result| {
                    let Ok(r) = result else { return false };
                    if !matches!(me.route(), Route::Library(Section::Tracks)) {
                        return false;
                    }
                    me.selection_pick_all(r.tracks);
                    true
                },
            );
            return false;
        }
        let Some(tracks) = self.table_tracks() else { return false };
        self.selection_pick_all(tracks);
        true
    }

    /// `tracks` : toute la liste, du rang 0.
    fn selection_pick_all(&mut self, tracks: Vec<TrackListRow>) {
        let s = self.views.selection.get_or_insert_with(Selection::default);
        s.picked = tracks.into_iter().enumerate().collect();
        s.all = true;
    }

    pub(crate) fn selection_update(&mut self, msg: SelectionMsg) -> bool {
        match msg {
            SelectionMsg::Toggle => {
                self.views.selection = if self.views.selection.is_some() { None } else { Some(Selection::default()) };
            }
            SelectionMsg::Stop => self.views.selection = None,
            SelectionMsg::SelectAll => return self.selection_select_all(),
            SelectionMsg::DeselectAll => {
                if let Some(s) = self.views.selection.as_mut() {
                    s.picked.clear();
                    s.all = false;
                }
            }
            SelectionMsg::PlaylistMenu => {
                if let Some(s) = self.views.selection.as_mut() {
                    s.playlists_open = !s.playlists_open;
                }
            }
            SelectionMsg::Play => {
                let tracks = self.selection_tracks();
                let Some(first) = tracks.first().map(|t| t.path.clone()) else { return false };
                let profile = self.data.profile_id().unwrap_or(0);
                self.views.selection = None;
                self.play_list(to_queue(profile, &tracks), &first);
            }
            SelectionMsg::Queue => {
                let tracks = self.selection_tracks();
                if tracks.is_empty() {
                    return false;
                }
                self.enqueue_views(&tracks);
                self.views.selection = None;
                self.toast(ToastKind::Success, t("notify.queued"), Some(count_label(tracks.len(), "notify.queued_one", "notify.queued_n")));
            }
            SelectionMsg::AddTo(id) => {
                let tracks = self.selection_tracks();
                let name = self.data.playlists.iter().find(|p| p.id == id).map(|p| p.name.clone()).unwrap_or_default();
                self.views.selection = None;
                let app = self.backend.app.clone();
                let items: Vec<(String, String)> = tracks.iter().map(|t| (t.id.clone(), t.path.clone())).collect();
                self.backend.call(
                    async move {
                        let mut added = 0;
                        for (track_id, path) in items {
                            if playlist_command::add_track_to_playlist(app.state(), id, Some(track_id), Some(path)).await.is_ok() {
                                added += 1;
                            }
                        }
                        added
                    },
                    move |me, added: usize| {
                        me.load_playlists();
                        me.toast(ToastKind::Success, t("notify.added"), Some(count_label(added, "notify.added_to_one", "notify.added_to_n").replace("{name}", &name)));
                        true
                    },
                );
            }
            SelectionMsg::EditTags => {
                let paths: Vec<String> = self.selection_tracks().into_iter().map(|t| t.path).collect();
                if paths.is_empty() {
                    return false;
                }
                self.views.selection = None;
                return self.batch_editor_update(crate::models::tags::BatchEditorMsg::Open(paths));
            }
            SelectionMsg::Workshop => {
                let paths: Vec<String> = self.selection_tracks().into_iter().map(|t| t.path).collect();
                if paths.is_empty() {
                    return false;
                }
                self.views.selection = None;
                self.views.workshop_paths = Some(paths);
                return self.handle(crate::models::app::Msg::Go(Route::Tags(None)));
            }
        }
        true
    }
}
