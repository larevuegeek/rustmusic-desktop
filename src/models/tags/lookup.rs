use crate::models::tags::*;
use crate::core::metadata_source::model::TrackHit;

#[derive(Debug, Clone)]
pub struct Lookup {
    pub query: String,
    pub musicbrainz: bool,
    pub hits: Vec<TrackHit>,
    pub searching: bool,
    pub searched: bool,
    pub loading: Option<usize>,
    pub error: Option<String>,
    pub seq: u64,
    /// Piste retenue : (résultat, lignes, cochées).
    pub picked: Option<(TrackHit, Vec<CompareRow>, Vec<&'static str>)>,
    pub cover: Option<Remote>,
    pub cover_loading: bool,
    pub cover_error: Option<String>,
    pub take_cover: bool,
}
