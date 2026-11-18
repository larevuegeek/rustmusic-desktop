use crate::models::player::StepTone;

/// Une étape de la chaîne audio, du fichier au DAC.
#[derive(Debug, Clone, PartialEq, Hash)]
pub struct PipelineStep {
    pub icon: &'static str,
    pub title: String,
    pub value: String,
    pub detail: Option<String>,
    /// Valeur technique, en chasse fixe.
    pub mono: bool,
    pub tone: StepTone,
}
