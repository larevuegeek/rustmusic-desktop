use crate::models::shared::ListWindow;
use crate::service::library::composer::ComposerView;

pub struct ArtistsPage {
    pub library_id: i64,
    pub sort: String,
    pub search: String,
    pub search_term: String,
    pub(crate) search_gen: u64,
    pub without_portrait: bool,
    /// Les éléments bâtis de la liste filtrée.
    pub window: ListWindow,
    pub composers: Vec<ComposerView>,
}
