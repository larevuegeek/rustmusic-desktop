#[derive(Debug, Clone, Default)]
pub struct WhatsNewState {
    /// Diapo affichée.
    pub index: usize,
    pub viewed: bool,
}
