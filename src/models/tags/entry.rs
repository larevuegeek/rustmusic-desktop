#[derive(Debug, Clone)]
pub struct Entry {
    pub id: String,
    pub kind: String,
    pub pattern: Option<String>,
    pub created_at: String,
    pub succeeded: i64,
    pub failed: i64,
    pub undone: bool,
}
