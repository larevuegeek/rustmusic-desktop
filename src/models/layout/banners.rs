use crate::models::layout::*;

#[derive(Debug, Clone, Default)]
pub struct Banners {
    pub support: bool,
    pub support_handled: bool,
    pub update: Update,
    /// Mise à jour annoncée, à télécharger sur demande.
    pub pending: Option<karbon_updater::Update>,
    /// Une vérification est en cours.
    pub checking: bool,
    pub last_check: crate::models::updates::LastCheck,
}
