use crate::models::artists::ArtistItem;

/// Une entrée triable : nom, albums, titres, durée, portrait.
pub(crate) struct Entry {
    pub(crate) item: ArtistItem,
    pub(crate) key: String,
    pub(crate) letter: &'static str,
    pub(crate) name: String,
    pub(crate) albums: i64,
    pub(crate) tracks: i64,
    pub(crate) duration: f64,
    pub(crate) portrait: bool,
}
