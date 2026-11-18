use crate::models::podcasts::*;

pub struct DownloadsView {
    pub loading: bool,
    pub empty: bool,
    pub episodes: (String, String),
    pub total: String,
    pub listens: String,
    pub recoverable: Option<String>,
    pub segments: Vec<StorageSegment>,
    pub confirm: Option<char>,
    pub delete_listens: Option<String>,
    pub groups: Vec<ShowGroup>,
}
