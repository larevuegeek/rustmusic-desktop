use crate::models::tags::*;

/// Une ligne de champ : clé, intitulé, case (0 non, 1 oui, 2 partiel), avant, après, effet
/// (texte, comble seulement), ouverte, détail par piste.
#[derive(Debug, Clone)]
pub struct FieldRow {
    pub field: String,
    pub label: String,
    pub check: u8,
    pub from: (String, u8),
    pub to: String,
    pub effect: (String, bool),
    pub open: bool,
    pub details: Vec<Detail>,
}
