use crate::models::artists::*;
use crate::models::albums::AlbumTrack;
use crate::models::albums::Carousel;
use crate::models::albums::SortChoice;
use crate::models::albums::AlbumItem;
use crate::models::artists::ArtistItem;
use crate::models::tracks::Table;

pub struct ArtistView {
    pub portrait: Option<String>,
    /// Teinte (degrés) et intensité (0..1) de la pochette : --h et --s de `.album-lueur`.
    pub tint: (f32, f32),
    pub name: String,
    pub name_long: bool,
    pub albums_n: Option<(String, String)>,
    pub tracks_n: (String, String),
    pub duration: String,
    pub period: Option<String>,
    pub liked_all: bool,
    pub has_tracks: bool,
    pub genres: Vec<String>,
    pub discography: Option<Carousel<AlbumItem>>,
    pub subtitle: String,
    pub sorts: Vec<SortChoice>,
    pub all_open: Option<bool>,
    pub loading: bool,
    pub groups: Vec<GroupView>,
    /// À plat : deux colonnes, ou la table.
    pub flat_left: Vec<AlbumTrack>,
    pub flat_right: Vec<AlbumTrack>,
    pub table: Option<Table>,
    pub head: Option<Table>,
    pub participations: Option<Carousel<AlbumItem>>,
    pub similar: Option<Carousel<ArtistItem>>,
}
