use crate::models::tracks::TrackMenuAt;
use crate::models::sidebar::PinKind;

/// Le menu ouvert d'un album ou d'un artiste.
#[derive(Debug)]
pub struct ItemMenu {
    pub at: TrackMenuAt,
    pub kind: PinKind,
    pub id: String,
    pub title: String,
    pub library_id: i64,
    pub pinned: bool,
    /// Artiste au nom douteux : la vérification est proposée.
    pub doubtful: bool,
}
