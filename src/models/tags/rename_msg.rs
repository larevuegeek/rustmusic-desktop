#[derive(Debug, Clone, PartialEq)]
pub enum RenameMsg {
    Open,
    Mode(bool),
    Root(u64),
    Satellites,
    Cleanup,
    Pattern(String),
    Insert(usize),
    Preset(usize),
    Toggle(String),
    SelectAll,
    Confirm,
    Cancel,
    Apply,
    Undo,
}
