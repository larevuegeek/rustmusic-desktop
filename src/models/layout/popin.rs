use crate::models::layout::*;

#[derive(Debug, Clone)]
pub struct Popin {
    pub title: String,
    pub icon: Option<&'static str>,
    pub body: PopinBody,
    pub size: PopinSize,
    /// `bare` : sans en-tête, le contenu porte sa propre fermeture.
    pub bare: bool,
    /// `flush` : sans marge ni défilement, le contenu gère les siens.
    pub flush: bool,
}
