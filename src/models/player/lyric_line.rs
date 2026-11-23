/// Une ligne de paroles synchronisées.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct LyricLine {
    /// Instant de la ligne (ms), décalage `[offset:]` appliqué.
    pub at: f64,
    pub text: String,
    /// Lignes sans horodatage qui suivent la ligne dans le fichier (traduction).
    pub translation: Vec<String>,
}
