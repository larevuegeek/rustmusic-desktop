use crate::models::app::Route;

/// Une tuile de navigation.
#[derive(Debug, Clone)]
pub struct Tile {
    pub key: &'static str,
    pub label: String,
    pub icon: &'static str,
    pub route: Route,
    pub active: bool,
}
