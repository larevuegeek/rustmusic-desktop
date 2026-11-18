use crate::models::playlists::*;
use std::collections::BTreeMap;

/// Le vocabulaire du moteur (champs et opérateurs), plus les tags recensés ajoutés aux champs.
#[derive(Debug, Clone, Default)]
pub struct Vocab {
    pub fields: Vec<Field>,
    pub ops: BTreeMap<String, Vec<Op>>,
}
