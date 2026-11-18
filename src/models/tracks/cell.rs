use crate::models::tracks::*;

/// Une cellule de ligne.
#[derive(Debug, Clone)]
pub struct Cell {
    pub key: String,
    pub widget: Widget,
    pub width: f32,
    pub flexible: bool,
    pub min_width: f32,
    pub text: String,
    pub right: bool,
    pub numeric: bool,
}
