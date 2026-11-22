use crate::models::app::Msg;
use crate::models::tracks::Tier;

/// Un sous-dossier ou un fichier.
pub struct EntryItem {
    pub name: String,
    pub path: String,
    pub is_dir: bool,
    pub covers: Vec<String>,
    pub albums: u32,
    pub album_badge: String,
    pub tracks: String,
    pub quality: Option<(Tier, String)>,
    pub music: bool,
    pub cover: Option<String>,
    pub format: String,
    pub tier: Tier,
    pub title: String,
    pub infos: String,
    pub size: String,
    pub open: Msg,
    /// En sélection, un fichier : coché ou non.
    pub checked: Option<bool>,
}
