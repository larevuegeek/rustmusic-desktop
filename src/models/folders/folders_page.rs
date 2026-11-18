use crate::entity::library::dir_entry::DirEntry;
use crate::models::shared::ListWindow;

#[derive(Default)]
pub struct FoldersPage {
    pub library_id: i64,
    pub path: Option<String>,
    pub breadcrumb: Vec<(String, String)>,
    pub entries: Vec<DirEntry>,
    pub loading: bool,
    pub search: String,
    pub sort: &'static str,
    pub without_covers: bool,
    /// Les éléments bâtis du dossier ouvert.
    pub window: ListWindow,
}
