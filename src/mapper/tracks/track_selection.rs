use crate::app::music_app::MusicApp;
use crate::query::track::track_list_row::TrackListRow;

impl crate::models::tracks::Selection {
    /// « 3 titres sélectionnés ».
    pub fn count_label(&self) -> String {
        crate::controller::tracks::track_selection::count_label(self.picked.len(), "actions.selected_one", "actions.selected_n")
    }
}

impl MusicApp {
    /// La sélection en cours, dès qu'un titre est coché.
    pub fn active_selection(&self) -> Option<&crate::models::tracks::Selection> {
        self.views.selection.as_ref().filter(|s| !s.picked.is_empty())
    }

    /// Le morceau de rang `i` est coché.
    pub fn selected(&self, i: usize) -> bool {
        self.views.selection.as_ref().is_some_and(|s| s.picked.contains_key(&i))
    }

    /// Toute la liste est cochée.
    pub fn all_selected(&self) -> bool {
        self.views.selection.as_ref().is_some_and(|s| s.all)
    }

    pub fn selecting(&self) -> bool {
        self.views.selection.is_some()
    }

    pub(crate) fn selection_tracks(&self) -> Vec<TrackListRow> {
        self.views.selection.as_ref().map(|s| s.picked.values().cloned().collect()).unwrap_or_default()
    }

    /// Les playlists où verser la sélection : (id, nom, icône, couleur, nombre).
    pub fn selection_playlists(&self) -> Vec<(i64, String, String, String, i64)> {
        self.data.playlists.iter().filter(|p| !p.is_smart && p.server_id.is_none()).map(|p| (p.id, p.name.clone(), p.icon.clone(), p.color.clone(), p.track_count)).collect()
    }
}
