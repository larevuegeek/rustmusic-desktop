use crate::models::profiles::*;

#[derive(Debug, Clone)]
pub struct ProfileScreen {
    pub mode: ProfileMode,
    pub name: String,
    pub color: usize,
    pub avatar: Option<String>,
    /// Clé de traduction de l'erreur.
    pub error: Option<&'static str>,
    pub busy: bool,
    pub delete_confirm: bool,
    pub delete_text: String,
}
