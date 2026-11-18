use crate::query::track::track_list_row::TrackListRow;

/// Un album de « Tous les titres ».
pub(crate) struct Group {
    pub(crate) key: String,
    pub(crate) album: Option<String>,
    pub(crate) title: String,
    pub(crate) year: Option<i32>,
    pub(crate) cover: Option<String>,
    pub(crate) tracks_list: Vec<TrackListRow>,
}
