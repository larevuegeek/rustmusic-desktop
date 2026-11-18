use crate::models::tags::*;
use std::collections::HashSet;

#[derive(Debug, Clone, Default)]
pub struct RenameState {
    pub restructure: bool,
    pub name_pattern: String,
    pub tree_pattern: String,
    pub dirs: Vec<String>,
    pub root: String,
    pub satellites: bool,
    pub cleanup: bool,
    /// Aperçu : déplacements, bloqués, déjà conformes.
    pub preview: Option<(Vec<Move>, usize, usize)>,
    pub pattern_error: Option<String>,
    pub computing: bool,
    pub excluded: HashSet<String>,
    pub confirming: bool,
    pub applying: bool,
    pub outcome: Option<Outcome>,
    pub undoing: bool,
    pub apply_error: Option<String>,
    pub seq: u64,
}
