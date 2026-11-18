#[derive(Debug, Clone, PartialEq)]
pub enum LookupMsg {
    Query(String),
    Search,
    Source(bool),
    Pick(usize),
    Back,
    Close,
    Toggle(&'static str),
    ToggleAll,
    TakeCover,
    Apply,
}
