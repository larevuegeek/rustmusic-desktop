use crate::models::tracks::TrackMenuAt;

#[derive(Debug, Clone, PartialEq)]
pub enum PodcastMenuMsg {
    /// Ouvre : un podcast suivi (id), ou une carte de l'annuaire (liste, rang).
    OpenFollowed(TrackMenuAt, i64),
    OpenDirectory(TrackMenuAt, char, usize),
    Preview,
    Go,
    Pin,
    Unfollow,
    Follow,
    Close,
}
