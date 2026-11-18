use crate::models::settings::*;

pub struct OptionGroup {
    pub key: String,
    pub title: Option<String>,
    pub hint: Option<String>,
    pub fold: Option<&'static str>,
    pub closed: bool,
    pub action: Option<GroupAction>,
    /// Aperçu de la barre latérale à droite des lignes : ouvrir, playlists, albums, artistes.
    pub sidebar: Option<[bool; 4]>,
    pub rows: Vec<OptionRow>,
}
