use crate::models::app::Msg;

/// Une case d'un panneau segmenté : icône ou jauge (1 à 4), teinte d'échelle.
#[derive(Debug, Clone)]
pub struct PanelChoice {
    pub label: String,
    pub icon: &'static str,
    pub level: u8,
    /// « green », « amber », « sky », « rose » ou vide.
    pub tone: &'static str,
    pub active: bool,
    pub msg: Msg,
}
