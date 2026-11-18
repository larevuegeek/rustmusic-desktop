#[derive(Debug, Clone, PartialEq)]
pub enum HistoryMsg {
    Open,
    Ask(String),
    Cancel,
    Undo(String),
}
