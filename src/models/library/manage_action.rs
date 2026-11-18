use crate::models::app::Msg;

/// Une action riche du menu « Gérer » : icône, titre, sous-titre, fin.
#[derive(Debug, Clone)]
pub struct ManageAction {
    pub icon: &'static str,
    pub title: String,
    pub sub: String,
    pub msg: Msg,
    /// Flèche de fin (mène à une autre page).
    pub arrow: bool,
    /// Compteur ambré de fin.
    pub count: Option<String>,
    pub alert: bool,
    pub busy: bool,
}
