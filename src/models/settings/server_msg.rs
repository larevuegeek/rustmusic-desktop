/// Une action sur un serveur.
#[derive(Debug, Clone, PartialEq)]
pub enum ServerMsg {
    Enabled(i64),
    PinPlaylists(i64),
    /// Indice dans `QUALITIES`.
    Quality(i64, u64),
    Sync(i64),
    AskRemove(i64),
    Remove(i64),
}
