#[derive(Debug, Clone, PartialEq)]
pub enum PodcastPreviewMsg {
    /// Depuis le menu d'un podcast.
    Open,
    Follow,
    OpenPage,
    ToggleDesc,
    Site,
}
