/// Une ligne de la liste de morceaux (gabarit de la liste virtualisée : sans `self`).
#[derive(Debug, Clone)]
pub struct TrackRow {
    pub n: usize,
    pub title: String,
    pub artist: String,
    pub album: String,
    pub duration: String,
}
