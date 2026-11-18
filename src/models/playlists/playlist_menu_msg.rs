use crate::models::tracks::TrackMenuAt;

#[derive(Debug, Clone, PartialEq)]
pub enum PlaylistMenuMsg {
    Open(TrackMenuAt, i64),
    Play,
    Edit,
    KeepOffline,
    Export,
    Pin,
    Delete,
    Close,
}
