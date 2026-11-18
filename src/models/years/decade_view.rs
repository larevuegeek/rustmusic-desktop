use crate::models::albums::AlbumItem;
use crate::models::genres::PeriodHead;

pub struct DecadeView {
    pub d: i64,
    pub short: String,
    pub hue: f32,
    pub name: String,
    pub albums_n: (String, String),
    pub tracks_n: (String, String),
    pub artists_n: (String, String),
    pub head: PeriodHead,
    pub mix: String,
    pub cards: Vec<AlbumItem>,
    pub start: usize,
    pub total: usize,
    pub see: String,
}
