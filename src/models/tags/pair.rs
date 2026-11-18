use crate::models::tags::*;
use crate::core::metadata_source::model::AlbumTrack;

#[derive(Debug, Clone)]
pub struct Pair {
    pub index: usize,
    pub path: String,
    pub has_cover: bool,
    pub track: AlbumTrack,
    pub doubtful: bool,
    pub cells: Vec<SourceCell>,
}
