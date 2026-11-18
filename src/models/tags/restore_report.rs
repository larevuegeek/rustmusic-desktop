/// Le bilan d'une restauration de l'historique.
pub struct RestoreReport {
    pub has_failures: bool,
    /// « 12 restaurés », ou « 10 restaurés · 2 échecs ».
    pub summary: String,
    /// Le détail de l'erreur, vide sans échec.
    pub detail: String,
}
