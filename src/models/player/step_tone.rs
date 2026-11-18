/// Teinte d'une étape de la chaîne audio : neutre, signal modifié (ambre), DSD natif
/// transporté (violet).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StepTone {
    Neutral,
    Amber,
    Violet,
}
