/// Une condition : listes des champs et des opérateurs, choix, valeurs saisies.
#[derive(Debug, Clone, Default)]
pub struct RuleView {
    pub path: Vec<usize>,
    pub fields: Vec<String>,
    pub field: u64,
    pub ops: Vec<String>,
    pub op: u64,
    pub arity: u8,
    pub value: String,
    pub low: String,
    pub high: String,
}
