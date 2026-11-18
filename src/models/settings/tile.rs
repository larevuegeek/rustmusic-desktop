use crate::models::app::Msg;

/// Titre, coche, puis explication.
#[derive(Debug, Clone)]
pub struct Tile {
    pub title: String,
    pub desc: String,
    pub selected: bool,
    pub msg: Msg,
}
