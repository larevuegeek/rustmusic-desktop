use crate::entity::podcast::podcast::PodcastPreview;

#[derive(Debug, Clone)]
pub struct PodcastPreviewState {
    pub feed_url: String,
    pub title: String,
    pub author: Option<String>,
    pub genre: Option<String>,
    pub released_at: Option<String>,
    /// Pochette locale (abonné, ou image de l'annuaire déjà téléchargée).
    pub image: Option<String>,
    pub preview: Option<PodcastPreview>,
    pub error: bool,
    pub open_desc: bool,
    pub adding: bool,
}
