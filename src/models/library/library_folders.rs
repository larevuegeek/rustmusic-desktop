use crate::entity::library::library_dirs::LibraryDir;

#[derive(Debug, Clone, Default)]
pub struct LibraryFolders {
    pub library_id: i64,
    pub dirs: Vec<LibraryDir>,
    pub loading: bool,
    pub rescanning: Option<String>,
}
