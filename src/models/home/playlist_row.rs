use crate::models::app::Route;

/// Une playlist de la barre ou de l'accueil (`PlaylistItem`, `PlaylistCard`).
#[derive(Debug, Clone)]
pub struct PlaylistRow {
    pub key: String,
    pub name: String,
    pub subtitle: String,
    pub color: String,
    pub icon: String,
    pub auto: bool,
    pub covers: Vec<String>,
    pub route: Route,
    pub active: bool,
}
