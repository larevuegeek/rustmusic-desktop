use crate::query::artist::track_artist_row::TrackArtistRow;
use crate::query::track::track_detail_row::TrackDetailRow;
use crate::query::track::track_list_row::TrackListRow;

/// La fiche ouverte.
pub struct TrackPage {
    pub id: String,
    pub loading_state: bool,
    pub track: Option<TrackDetailRow>,
    pub credits: Vec<TrackArtistRow>,
    /// Les titres de son album, dans l'ordre du disque.
    pub album: Vec<TrackListRow>,
}
