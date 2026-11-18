use crate::models::albums::*;
use crate::models::app::Route;
use crate::models::albums::AlbumItem;
use crate::models::artists::ArtistItem;
use crate::models::tracks::Table;
use crate::models::tracks::Tier;

pub struct AlbumView {
    pub cover: Option<String>,
    /// Teinte (degrés) et intensité (0..1) de la pochette : --h et --s de `.album-lueur`.
    pub tint: (f32, f32),
    pub type_label: String,
    pub quality: Option<(Tier, String)>,
    pub title: String,
    pub title_long: bool,
    pub artist: String,
    pub artist_route: Option<Route>,
    pub artist_portrait: Option<String>,
    pub year: Option<i32>,
    pub tracks_count: String,
    pub duration: String,
    pub weight: String,
    pub count_n: String,
    pub count_label: String,
    pub liked_all: bool,
    pub genres: Vec<String>,
    pub sorts: Vec<SortChoice>,
    pub multi: bool,
    pub discs: Vec<Disc>,
    pub table: Option<Table>,
    pub output: Option<String>,
    pub label: Option<String>,
    pub folder: Option<String>,
    pub reveal: String,
    pub notes: Option<String>,
    pub others: Option<Carousel<AlbumItem>>,
    pub similar: Option<Carousel<AlbumItem>>,
    pub artists: Option<Carousel<ArtistItem>>,
    pub prev: Option<Route>,
    pub next: Option<Route>,
}
