/// Où s'ouvre le menu : le pointeur, ou un élément nommé (le bouton « … »).
#[derive(Debug, Clone, PartialEq)]
pub enum TrackMenuAt {
    Point(f32, f32),
    Named(String),
}
