use crate::models::mixes::*;

pub struct MixesView {
    pub loading: bool,
    pub for_you: Vec<MixTile>,
    pub custom: Vec<MixTile>,
    pub genres: Vec<MixTile>,
    pub decades: Vec<MixTile>,
}
