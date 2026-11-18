#[derive(Debug, Clone, PartialEq)]
pub enum TrackMsg {
    Play,
    PlayNext,
    Like,
    Reveal,
    /// Note posée (le même cran l'efface).
    Rate(f64),
}
