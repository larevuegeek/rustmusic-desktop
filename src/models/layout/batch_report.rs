use crate::models::layout::*;

/// Le bilan d'un lot terminé : « 12 réussis · 1 échec · 3 ignorés », et le détail des échecs.
pub struct BatchReport {
    pub succeeded: String,
    pub failed: Option<String>,
    pub skipped: Option<String>,
    pub failures: Vec<BatchFailure>,
}
