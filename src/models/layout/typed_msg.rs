#[derive(Debug, Clone, PartialEq)]
pub enum TypedMsg {
    Reset,
    RemoveServer(i64),
    Input(String),
    Confirm,
    Cancel,
}
