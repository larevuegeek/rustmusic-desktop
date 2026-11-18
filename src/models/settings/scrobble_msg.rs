#[derive(Debug, Clone, PartialEq)]
pub enum ScrobbleMsg {
    LbToken(String),
    LbConnect,
    LbDisconnect,
    LbKeep,
    LbNowPlaying,
    LfKey(String),
    LfSecret(String),
    LfOwn,
    LfConnect,
    LfReopen,
    LfCancel,
    LfDisconnect,
    LfKeep,
    LfNowPlaying,
    /// Un lien vers le site du service.
    Open(String),
}
