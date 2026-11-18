use crate::models::playlists::*;
use crate::models::app::Choice;

pub struct PlaylistsView {
    pub count: (String, String),
    pub pinned: (String, String),
    pub placeholder: String,
    pub kinds: Vec<Choice>,
    pub sort_label: String,
    pub sort_icon: &'static str,
    pub sorts: Vec<Choice>,
    pub views: Vec<Choice>,
    pub filtres: bool,
    /// (titre de groupe, playlists) : « Dans RustMusic » et « Sur le serveur » s'il y a les deux.
    pub groups: Vec<(Option<(String, &'static str)>, Vec<PlaylistTile>)>,
    pub shortcuts: Vec<PlaylistTile>,
    pub empty: bool,
    pub no_match: bool,
    pub remote: bool,
}
