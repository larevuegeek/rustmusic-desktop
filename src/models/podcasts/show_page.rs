use crate::entity::podcast::podcast::{EpisodeView, PodcastView};
use crate::models::shared::ListWindow;

pub struct ShowPage {
    pub id: i64,
    pub podcast: Option<PodcastView>,
    pub episodes: Vec<EpisodeView>,
    pub state: &'static str,
    pub unplayed_only: bool,
    pub older: bool,
    /// Les épisodes bâtis de la liste filtrée.
    pub window: ListWindow,
    pub description: bool,
    pub open_items: Vec<(i64, Option<String>)>,
    pub downloads: Vec<i64>,
    pub verification: bool,
    pub confirm: bool,
    pub menu_episode: Option<i64>,
}
