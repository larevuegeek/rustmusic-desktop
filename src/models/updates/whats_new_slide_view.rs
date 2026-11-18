use crate::models::updates::*;

/// Une diapo des nouveautés, textes traduits.
pub struct WhatsNewSlideView {
    pub index: usize,
    pub slide: &'static Slide,
    /// La diapo affichée (animations en marche).
    pub active: bool,
    /// « 2 / 6 ».
    pub position: String,
    pub title: String,
    pub text: String,
}
