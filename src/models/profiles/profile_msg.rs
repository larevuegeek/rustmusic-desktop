#[derive(Debug, Clone, PartialEq)]
pub enum ProfileMsg {
    Open,
    Close,
    Select(i64),
    Create,
    Edit(i64),
    Back,
    Name(String),
    Color(usize),
    /// Choisir l'image du profil sur l'ordinateur.
    ChooseAvatar,
    /// Prendre l'avatar proposé de rang donné (`AVATAR_PRESETS`).
    PresetAvatar(usize),
    RemoveAvatar,
    Submit,
    DeleteAsk,
    DeleteText(String),
    DeleteCancel,
    Delete,
}
