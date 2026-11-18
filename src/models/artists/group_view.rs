use crate::models::app::Route;
use crate::models::albums::AlbumTrack;
use crate::models::tracks::Table;
use crate::models::tracks::Tier;

/// L'en-tête repliable d'un groupe et ses titres.
pub struct GroupView {
    pub key: String,
    pub title: String,
    pub year: Option<String>,
    pub tracks_count: String,
    pub duration: String,
    pub quality: Option<(Tier, String)>,
    pub cover: Option<String>,
    pub album_route: Option<Route>,
    pub open: bool,
    /// Marge au-dessus : `mt-2` (premier), `mt-7` (après un groupe ouvert), `mt-1`.
    pub space: u8,
    pub left: Vec<AlbumTrack>,
    pub right: Vec<AlbumTrack>,
    pub table: Option<Table>,
}
