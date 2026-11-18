use crate::models::app::Msg;

/// Une carte de l'annuaire.
#[derive(Debug, Clone)]
pub struct DirectoryItem {
    pub key: String,
    pub title: String,
    pub sub: String,
    pub image: Option<String>,
    pub rank: Option<usize>,
    pub tracked: bool,
    pub busy: bool,
    pub open: Msg,
    pub follow: Msg,
    /// Liste (`r`, `p`) et rang, pour le menu.
    pub list: char,
    pub index: usize,
}
