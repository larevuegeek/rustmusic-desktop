use crate::models::folders::*;
use crate::models::app::Choice;

pub struct FoldersView {
    pub is_root: bool,
    pub roots: Vec<RootItem>,
    pub dirs: Vec<EntryItem>,
    pub files: Vec<EntryItem>,
    pub crumbs: Vec<Crumb>,
    pub dir_count: Option<String>,
    pub file_count: Option<String>,
    pub has_files: bool,
    pub empty_folder: bool,
    pub no_match: bool,
    pub placeholder: String,
    pub sort_label: String,
    pub sort_icon: &'static str,
    pub sorts: Vec<Choice>,
    pub views: Vec<Choice>,
    pub music_here: bool,
}
