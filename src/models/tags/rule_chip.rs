/// Une règle de nettoyage, à cocher.
pub struct RuleChip {
    /// Rang (`CleanMsg::Toggle`).
    pub index: usize,
    pub label: String,
    pub active: bool,
}
