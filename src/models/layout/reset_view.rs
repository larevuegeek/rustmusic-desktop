use crate::models::layout::*;

/// Tout réinitialiser : l'avertissement et ce qui sera effacé.
pub struct ResetView {
    pub intro: BoldLine,
    pub items: Vec<BoldLine>,
}
