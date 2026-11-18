#[derive(Debug, Clone, PartialEq)]
pub enum CoverMsg {
    /// Album, requête de départ (« artiste titre »).
    Open(String, String),
    Query(String),
    Search,
    Select(usize),
    Apply,
    Close,
}
