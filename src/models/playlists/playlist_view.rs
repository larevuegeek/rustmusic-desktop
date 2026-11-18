use crate::models::albums::AlbumTrack;
use crate::models::albums::SortChoice;
use crate::models::tracks::Table;

pub struct PlaylistView {
    pub title: String,
    pub title_long: bool,
    pub kicker: String,
    pub color: String,
    pub icon: String,
    /// Teinte (degrés) et intensité (0..1) de la pochette : --h et --s de `.album-lueur`.
    pub tint: (f32, f32),
    pub covers: Vec<String>,
    pub count: (String, String),
    pub duration: String,
    pub sorts: Vec<SortChoice>,
    pub left: Vec<AlbumTrack>,
    pub right: Vec<AlbumTrack>,
    pub table: Option<Table>,
    pub empty: (String, String),
    pub remove: Option<String>,
    pub playlist_id: Option<i64>,
    pub pinned: bool,
    pub smart: bool,
    pub confirm: Option<(String, String)>,
}
