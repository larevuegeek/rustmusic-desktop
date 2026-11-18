use crate::models::app::Msg;

/// Une ligne de la liste affichée par l'ampli (écran des services réseau).
pub struct AmpNetLine {
    pub text: String,
    pub icon: &'static str,
    pub selected: bool,
    /// Dossier ou service : un chevron en bout de ligne.
    pub folder: bool,
    pub msg: Msg,
}
