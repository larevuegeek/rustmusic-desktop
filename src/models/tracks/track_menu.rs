use crate::models::tracks::*;
use crate::query::track::track_list_row::TrackListRow;

/// Le morceau du menu ouvert.
pub struct TrackMenu {
    pub at: TrackMenuAt,
    pub track: TrackListRow,
    pub sub: bool,
    pub nav: bool,
}
