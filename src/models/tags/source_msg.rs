#[derive(Debug, Clone, PartialEq)]
pub enum SourceMsg {
    Open,
    Source(bool),
    Query(String),
    Search,
    Pick(String),
    Step1,
    Next,
    Mode(u8),
    ToggleField(String),
    ToggleCover,
    ToggleCell(String),
    Open1(String),
    MapOpen,
    Exclude(usize),
    Apply,
    /// Aperçu hors écran : enchaîner sur l'étape 2.
    Auto,
}
