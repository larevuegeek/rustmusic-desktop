use crate::query::album::album_detail_row::AlbumDetailRow;
use crate::query::track::track_list_row::TrackListRow;

pub struct AlbumPage {
    pub id: String,
    pub album: Option<AlbumDetailRow>,
    pub tracks: Vec<TrackListRow>,
    pub loading_state: bool,
    pub error: bool,
    pub sort: Option<String>,
    pub asc: bool,
    pub others_by_year: bool,
    pub similar: Vec<String>,
    pub similar_artists: Vec<String>,
}
