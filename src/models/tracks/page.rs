use crate::models::tracks::*;
use std::rc::Rc;
use crate::models::app::Route;
use crate::entity::queue::queue_track::QueueTrack;

/// La page ouverte et ses données.
pub struct Page {
    pub route: Route,
    pub kind: PageKind,
    pub kicker: String,
    pub title: String,
    pub subtitle: String,
    pub cover: Option<String>,
    pub rows: Rc<Vec<TrackRow>>,
    /// Ce que la file devient quand on lance un morceau de la page.
    pub queue: Vec<QueueTrack>,
}
