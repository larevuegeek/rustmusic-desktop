use crate::models::app::Msg;

/// Le bloc d'un compte relié : service, texte, boutons.
pub struct AccountView {
    pub lastfm: bool,
    pub user: String,
    /// En attente de confirmation de la déconnexion : l'avertissement.
    pub warning: Option<String>,
    pub sub: String,
    pub profile: Msg,
    pub buttons: Vec<crate::models::settings::Ghost>,
}
