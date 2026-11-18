/// Un podcast suivi.
#[derive(Debug, Clone)]
pub struct PodcastItem {
    pub id: i64,
    pub title: String,
    pub author: String,
    pub cover: Option<String>,
    pub badge: Option<String>,
    pub pinned: bool,
}
