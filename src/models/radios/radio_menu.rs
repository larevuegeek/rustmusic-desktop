use crate::models::tracks::TrackMenuAt;
use crate::core::radio::directory::StreamVariant;

pub struct RadioMenu {
    pub at: TrackMenuAt,
    pub list: char,
    pub i: usize,
    pub name: String,
    pub favorite: bool,
    pub homepage: Option<String>,
    pub current: String,
    pub variants: Vec<StreamVariant>,
}
