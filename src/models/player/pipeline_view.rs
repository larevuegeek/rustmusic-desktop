use crate::models::player::PipelineStep;

/// Le détail de la chaîne audio, ouvert d'un clic sur la qualité dans la barre du lecteur.
#[derive(Debug, Clone, PartialEq, Hash)]
pub struct PipelineView {
    /// Mode de la chaîne (`bit-perfect`, `exclusive`, `dop`, `resampled`, `dsd`, `shared`).
    pub mode: &'static str,
    pub label: String,
    pub description: String,
    /// « DSD64 · 2,82 MHz → 48 kHz » : du fichier à la sortie.
    pub summary: String,
    /// Le détail en texte, pour le presse-papiers.
    pub clipboard: String,
    /// Sortie demandée non obtenue, en clair.
    pub fallback: Option<String>,
    pub steps: Vec<PipelineStep>,
    /// Profil de qualité actif.
    pub profile: Option<String>,
}
