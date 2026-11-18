#[derive(Debug, Clone, PartialEq)]
pub enum CleanMsg {
    Open,
    Toggle(usize),
    Apply,
}
