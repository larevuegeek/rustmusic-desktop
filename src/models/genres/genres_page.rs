use crate::models::shared::ListWindow;

pub struct GenresPage {
    pub library_id: i64,
    pub sort: String,
    pub search: String,
    pub main_items: bool,
    pub without_genre: bool,
    /// Les éléments bâtis de la liste filtrée.
    pub window: ListWindow,
}
