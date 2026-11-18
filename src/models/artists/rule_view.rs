#[derive(Debug, Clone, PartialEq)]
pub struct RuleView {
    pub name: String,
    pub parts: Vec<String>,
    pub source: String,
}
