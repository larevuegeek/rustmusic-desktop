use crate::entity::library::genre_view::GenreView;
use crate::query::track::track_list_row::TrackListRow;

/// L'état de la page.
pub struct TracksPage {
    pub library_id: i64,
    pub sort: String,
    pub asc: bool,
    pub detailed: bool,
    pub search: String,
    pub sent: String,
    pub(crate) search_gen: u64,
    pub hires: bool,
    pub lossless: bool,
    pub favorites: bool,
    pub without_cover: bool,
    pub genre: Option<String>,
    pub genres: Option<Vec<GenreView>>,
    pub tracks: Vec<TrackListRow>,
    /// Lot reçu dont les lignes entrent dans `tracks` par tranches (`tracks_frame_work`).
    pub(crate) a_venir: Vec<TrackListRow>,
    /// Lot d'avant la fenêtre (défilement vers le haut) : son premier rang.
    pub(crate) upcoming_start: Option<i64>,
    pub start: i64,
    pub total: i64,
    pub loading_state: bool,
    pub following: bool,
    pub(crate) token: u64,
    /// Génération de la zone défilante : une nouvelle zone (nouvelle liste, saut à une
    /// lettre) part de son décalage initial.
    pub view: u64,
    /// Index absolu à amener en haut de la nouvelle zone.
    pub anchor: Option<i64>,
    pub letters: Vec<(String, i64)>,
}
