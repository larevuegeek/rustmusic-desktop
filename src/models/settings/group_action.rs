use crate::models::settings::*;

/// À droite de l'intitulé d'un groupe : badge ou bouton.
#[derive(Debug, Clone)]
pub enum GroupAction {
    /// Texte et ton (« amber », « green », « neutral »).
    Badge(String, &'static str),
    Button(Ghost),
}
