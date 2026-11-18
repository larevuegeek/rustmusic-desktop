use std::collections::HashSet;
use crate::query::album::album_list_row::AlbumListRow;
use crate::query::artist::artist_detail_row::ArtistDetailRow;
use crate::query::artist::artist_list_row::ArtistListRow;
use crate::query::track::track_list_row::TrackListRow;

pub struct ArtistPage {
    pub id: String,
    pub artist: Option<ArtistDetailRow>,
    pub portrait: Option<String>,
    pub tracks: Vec<TrackListRow>,
    pub albums: Vec<AlbumListRow>,
    pub similar: Vec<ArtistListRow>,
    pub loading_tracks: bool,
    pub loading_albums: bool,
    pub error: bool,
    pub sort: String,
    pub asc: bool,
    pub open_items: HashSet<String>,
}
