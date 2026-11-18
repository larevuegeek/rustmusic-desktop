use crate::models::mixes::*;

/// Un mix et son tirage.
#[derive(Debug, Clone)]
pub struct MixTile {
    pub key: String,
    pub kind: String,
    pub name: String,
    pub help: String,
    pub h: f32,
    pub source: MixSource,
    pub pre: String,
    pub short: String,
    pub playlist: Option<i64>,
    pub covers_list: Vec<String>,
    pub description: String,
    pub meta: String,
    pub ready: bool,
    pub menu: bool,
    /// Variante `decade` : le millésime en grand.
    pub decade: bool,
}
