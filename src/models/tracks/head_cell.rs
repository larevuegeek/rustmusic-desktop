use crate::models::tracks::*;

/// Une cellule d'en-tête.
#[derive(Debug, Clone)]
pub struct HeadCell {
    pub key: String,
    pub label: String,
    pub widget: Widget,
    pub width: f32,
    pub flexible: bool,
    pub min_width: f32,
    pub right: bool,
    pub sorted: bool,
    pub asc: bool,
}
