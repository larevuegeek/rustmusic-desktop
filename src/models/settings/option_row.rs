use crate::models::settings::*;

#[derive(Debug, Clone)]
pub struct OptionRow {
    pub key: String,
    pub title: String,
    pub desc: String,
    /// Valeur technique (chemin, adresse) en petite ligne mono.
    pub value: Option<String>,
    /// Pastille avant le texte : icône et teinte.
    pub lead: Option<(&'static str, i32)>,
    pub control: Control,
    pub disabled: bool,
}
