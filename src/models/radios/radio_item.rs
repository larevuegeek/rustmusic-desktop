use crate::models::app::Msg;

/// Une carte de station.
#[derive(Debug, Clone)]
pub struct RadioItem {
    pub key: String,
    pub name: String,
    pub sub: String,
    pub format: String,
    pub logo: Option<String>,
    pub rank: Option<usize>,
    pub reduced: bool,
    pub favorite: bool,
    pub play: Msg,
    pub fav: Msg,
    /// Liste (`r`, `p`, `f`, `h`) et rang, pour le menu.
    pub list: char,
    pub index: usize,
}
