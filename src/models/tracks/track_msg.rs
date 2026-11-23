#[derive(Debug, Clone, PartialEq)]
pub enum TrackMsg {
    Play,
    PlayNext,
    Like,
    Reveal,
    /// Menu « … » de la fiche.
    Menu,
    /// Note posée (le même cran l'efface).
    Rate(f64),
}
