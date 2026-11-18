use crate::models::albums::*;

pub struct Disc {
    pub n: i32,
    pub label: String,
    pub left: Vec<AlbumTrack>,
    pub right: Vec<AlbumTrack>,
}
