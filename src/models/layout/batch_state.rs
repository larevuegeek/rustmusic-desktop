#[derive(Default)]
pub struct BatchState {
    pub job_id: String,
    pub label: String,
    pub running: bool,
    pub cancelling: bool,
    pub done: usize,
    pub total: usize,
    pub failed: usize,
    pub current: String,
    /// Compte rendu : réussis, échecs (fichier, erreur), ignorés.
    pub report: Option<(usize, Vec<(String, String)>, usize)>,
}
