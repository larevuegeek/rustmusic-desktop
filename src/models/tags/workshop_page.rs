use crate::models::tags::*;
use std::collections::HashSet;

#[derive(Default)]
pub struct WorkshopPage {
    pub album: Option<String>,
    pub source: String,
    pub loading: bool,
    pub files: Vec<WorkshopFile>,
    pub selected: HashSet<String>,
    pub undo: Vec<Vec<WorkshopFile>>,
    /// Colonnes montrées (indices, `FILE_COLUMN` pour le fichier) ; le titre l'est toujours.
    pub shown: Vec<usize>,
    pub panel: bool,
    pub only_issues: bool,
    pub editing: Option<(String, usize)>,
    pub draft: String,
    pub sort: Option<(usize, bool)>,
    pub menu: Option<String>,
    pub drafts: std::collections::HashMap<usize, String>,
    pub notice: Option<Notice>,
    pub next_notice: u64,
    pub launching: bool,
    pub error: Option<String>,
    pub confirm_leave: bool,
}
