/// Un épisode à écouter.
#[derive(Debug, Clone)]
pub struct EpisodeCardItem {
    pub id: i64,
    pub podcast_id: i64,
    pub podcast_title: String,
    pub title: String,
    pub cover: Option<String>,
    pub badge: Option<String>,
    pub progress: Option<f32>,
    pub meta: String,
    pub playing: bool,
}
