use crate::models::player::*;

#[derive(Default)]
pub struct LyricsState {
    /// Le morceau dont les paroles sont chargées.
    pub path: Option<String>,
    pub status: LyricsStatus,
    pub error: Option<String>,
    pub plain: Option<String>,
    /// Lignes synchronisées : instant (ms), texte.
    pub lines: Vec<(f64, String)>,
    pub source: String,
    pub refreshing: bool,
}
