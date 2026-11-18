/// Le lecteur de la zone de notification.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrayMsg {
    Previous,
    TogglePlay,
    Next,
    /// Ouvre la fenêtre de RustMusic (et ferme le petit lecteur).
    Open,
}
