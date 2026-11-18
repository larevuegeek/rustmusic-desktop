use crate::models::albums::*;

#[derive(Debug, Clone)]
pub struct CoverSearch {
    pub album_id: String,
    pub query: String,
    pub results: Vec<CoverHit>,
    pub loading: bool,
    pub applying: bool,
    pub selected: Option<usize>,
    pub seq: u64,
}
