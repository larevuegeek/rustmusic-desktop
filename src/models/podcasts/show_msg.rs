#[derive(Debug, Clone, PartialEq)]
pub enum ShowMsg {
    Play(i64),
    PlayFirst,
    Refresh,
    Pin,
    AutoDownload,
    SubscriptionMenu,
    ConfirmUnsubscribe(bool),
    Unsubscribe,
    MarkAllPlayed,
    Order,
    Filter(bool),
    /// Défilement de la fiche : décalage et décalage maximal (fenêtre qui glisse).
    Scrolled(f32, f32),
    Description,
    Expand(i64),
    TogglePlayed(i64),
    Download(i64),
    DeleteDownload(i64),
    EpisodeMenu(i64),
}
