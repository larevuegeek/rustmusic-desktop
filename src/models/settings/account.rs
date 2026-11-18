/// Un compte relié.
#[derive(Debug, Clone)]
pub struct Account {
    pub user: String,
    pub now_playing: bool,
    pub pending: i64,
    pub own_key: bool,
}
