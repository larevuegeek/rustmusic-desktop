#[derive(Debug, Clone, PartialEq)]
pub enum BannerMsg {
    /// Au lancement.
    Start,
    /// Fin d'une attente : montrer la carte si rien ne l'empêche.
    SupportDue,
    SupportLater,
    SupportNever,
    SupportOpen,
    UpdateLater,
    /// « Vérifier » (réglages, À propos) : vérification qui affiche aussi les erreurs.
    UpdateCheck,
    /// « Installer maintenant ».
    UpdateInstall,
}
