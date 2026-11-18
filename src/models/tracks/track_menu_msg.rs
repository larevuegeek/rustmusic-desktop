#[derive(Debug, Clone, PartialEq)]
pub enum TrackMenuMsg {
    Play,
    PlayNext,
    Enqueue,
    Like,
    Sub,
    AddTo(i64),
    NewPlaylist,
    Track,
    Album,
    Artist,
    Tags,
    Close,
}
