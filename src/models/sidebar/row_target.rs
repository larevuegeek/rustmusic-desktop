use crate::models::sidebar::*;

/// Ce que désigne une ligne de la barre, pour son menu.
#[derive(Debug, Clone, PartialEq)]
pub enum RowTarget {
    Playlist(i64),
    Pin(PinKind, String),
}
