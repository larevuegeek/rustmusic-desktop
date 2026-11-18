use crate::models::app::Route;

/// Une carte ou une ligne.
#[derive(Debug, Clone)]
pub struct PlaylistTile {
    pub key: String,
    pub name: String,
    pub subtitle: String,
    pub color: String,
    pub icon: String,
    pub auto: bool,
    pub covers: Vec<String>,
    pub route: Route,
    /// `None` : pas de punaise (Titres aimés, Récents).
    pub pinned: Option<(i64, bool)>,
}
