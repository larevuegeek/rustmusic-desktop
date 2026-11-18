use std::collections::BTreeMap;
use crate::query::track::track_list_row::TrackListRow;

#[derive(Debug, Clone, Default)]
pub struct Selection {
    /// Morceaux cochés, par rang dans la liste de la page.
    pub picked: BTreeMap<usize, TrackListRow>,
    /// Le menu « Playlist » de la barre.
    pub playlists_open: bool,
    /// Toute la liste est cochée (« Tout sélectionner »), jusqu'au prochain décochage.
    pub all: bool,
}
