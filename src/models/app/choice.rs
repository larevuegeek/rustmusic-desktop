use crate::models::app::Msg;

/// Une option de `MenuSelect` ou de `SegmentedControl`.
#[derive(Debug, Clone)]
pub struct Choice {
    pub label: String,
    pub icon: String,
    pub active: bool,
    pub msg: Msg,
    /// Bouton carré, libellé en info-bulle (`SegmentedControl`).
    pub icon_only: bool,
}
