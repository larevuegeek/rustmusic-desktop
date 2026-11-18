use crate::models::library::*;
use crate::models::tracks::TrackMenuAt;

pub struct CollectionMenu {
    pub at: TrackMenuAt,
    pub title: String,
    pub source: CollectionSource,
    pub sub_playlists: bool,
    pub sub_cover: bool,
    /// Épinglé dans la barre latérale (lu à l'ouverture).
    pub pinned: Option<bool>,
}
