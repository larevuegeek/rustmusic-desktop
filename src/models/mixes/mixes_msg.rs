#[derive(Debug, Clone, PartialEq)]
pub enum MixesMsg {
    Play(String),
    Redraw(Option<String>),
    Keep(String),
    Delete(i64),
    Menu(String),
}
