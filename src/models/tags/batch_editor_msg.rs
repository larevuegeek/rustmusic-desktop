#[derive(Debug, Clone, PartialEq)]
pub enum BatchEditorMsg {
    Open(Vec<String>),
    Field(usize, String),
    Revert(usize),
    RevertAll,
    Numbering,
    Submit,
    Close,
    Discard,
    KeepEditing,
}
