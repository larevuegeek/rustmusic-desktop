pub struct InspectorField {
    pub idx: usize,
    pub label: String,
    pub value: String,
    pub placeholder: String,
    pub half: bool,
    /// Propre à chaque morceau (titre, numéro) quand plusieurs sont choisis.
    pub locked: bool,
    pub missing: bool,
    pub dirty: bool,
}
