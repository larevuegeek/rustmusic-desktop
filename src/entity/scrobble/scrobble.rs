use sqlx::FromRow;

/// Compte de suivi d'écoute d'un profil (ListenBrainz, Last.fm).
#[derive(Debug, Clone, FromRow)]
pub struct ScrobbleAccount {
    pub service: String,
    /// Jeton ListenBrainz, ou clé de session Last.fm.
    pub token: String,
    pub user_name: String,
    /// Annoncer le morceau en cours.
    pub now_playing: bool,
    /// Last.fm : la clé d'API de l'utilisateur, s'il a choisi la sienne.
    pub api_key: Option<String>,
    pub api_secret: Option<String>,
}
