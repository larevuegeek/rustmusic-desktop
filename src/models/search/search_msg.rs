#[derive(Debug, Clone, PartialEq)]
pub enum SearchMsg {
    Filter(&'static str),
    Open(usize),
    PlayAll,
}
