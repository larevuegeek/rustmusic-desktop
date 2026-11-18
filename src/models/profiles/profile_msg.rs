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
    RemoveAvatar,
    Submit,
    DeleteAsk,
    DeleteText(String),
    DeleteCancel,
    Delete,
}
