/// Ancre du « … » d'une fiche (album, artiste, genre, morceau) : un seul par page.
pub const DETAIL_MORE: &str = "detail.more";

/// Où s'ouvre le menu : le pointeur, ou un élément nommé (le bouton « … »).
#[derive(Debug, Clone, PartialEq)]
pub enum TrackMenuAt {
    Point(f32, f32),
    Named(String),
}
