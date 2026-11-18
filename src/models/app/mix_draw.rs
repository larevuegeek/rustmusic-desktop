use crate::query::track::track_list_row::TrackListRow;

/// Un tirage de mix (`draw`), par clé de mix.
#[derive(Debug, Clone, Default)]
pub struct MixDraw {
    pub tracks: Vec<TrackListRow>,
    pub loaded: bool,
}
