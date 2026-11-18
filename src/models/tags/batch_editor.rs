use std::collections::HashSet;

#[derive(Debug, Clone, Default)]
pub struct BatchEditor {
    pub paths: Vec<String>,
    /// Valeur commune de chaque champ ; `None` : valeurs multiples.
    pub common: Vec<Option<String>>,
    pub form: Vec<String>,
    pub touched: HashSet<usize>,
    pub numbering: bool,
    pub loading: bool,
    pub busy: bool,
    pub error: Option<String>,
    pub confirming: bool,
}
