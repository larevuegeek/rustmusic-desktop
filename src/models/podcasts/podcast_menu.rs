use crate::models::tracks::TrackMenuAt;

pub struct PodcastMenu {
    pub at: TrackMenuAt,
    pub title: String,
    /// Le podcast suivi : id, favori.
    pub followed: Option<(i64, bool)>,
    pub directory: Option<(char, usize)>,
    pub confirm: bool,
}
