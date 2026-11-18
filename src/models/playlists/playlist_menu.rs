use crate::models::tracks::TrackMenuAt;

pub struct PlaylistMenu {
    pub at: TrackMenuAt,
    pub id: i64,
    pub name: String,
    pub smart: bool,
    pub pinned: bool,
    pub server: bool,
    /// `source_id` : playlist d'une source distante (pas d'export .m3u).
    pub remote: bool,
    pub confirm: bool,
}
