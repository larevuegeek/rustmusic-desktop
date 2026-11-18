use crate::models::tags::*;

#[derive(Debug, Clone)]
pub struct TagEditor {
    pub path: String,
    pub original: Vec<String>,
    pub form: Vec<String>,
    pub slots: Vec<Slot>,
    pub original_slots: Vec<Slot>,
    pub zoomed: Option<usize>,
    pub loading: bool,
    pub busy: bool,
    pub error: Option<String>,
    pub confirming: bool,
    /// Champs repris de Deezer, nommés une fois.
    pub filled: Vec<String>,
    /// La recherche Deezer, quand elle occupe la fenêtre.
    pub lookup: Option<crate::models::tags::Lookup>,
}
