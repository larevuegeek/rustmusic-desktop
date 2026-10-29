use sqlx::FromRow;

/// Compte de suivi d'écoute d'un profil (ListenBrainz).
#[derive(Debug, Clone, FromRow)]
pub struct ScrobbleAccount {
    pub token: String,
    pub user_name: String,
    /// Annoncer le morceau en cours.
    pub now_playing: bool,
}
