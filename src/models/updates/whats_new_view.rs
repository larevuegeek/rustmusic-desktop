use crate::models::updates::*;

/// La fenêtre des nouveautés (`whatsnew/WhatsNewPopin.ui`).
pub struct WhatsNewView {
    /// Diapo affichée.
    pub index: usize,
    /// « Nouveau dans la 0.4.4 », ou le rappel d'une version précédente.
    pub kicker: String,
    pub slides: Vec<WhatsNewSlideView>,
    /// La diapo affichée (couleurs du bouton « Suivant »).
    pub current: &'static Slide,
    /// Diapo précédente (bouton « ‹ »), absente sur la première.
    pub previous: Option<usize>,
    /// Dernière diapo : « Terminé » au lieu de « Suivant ».
    pub last: bool,
    /// « Afficher au démarrage » coché.
    pub at_startup: bool,
}
