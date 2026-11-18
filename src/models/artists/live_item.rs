#[derive(Debug, Clone, PartialEq)]
pub struct LiveItem {
    pub name: String,
    /// « split », « whole » ou « unknown ».
    pub decision: String,
    pub parts: Vec<String>,
}
