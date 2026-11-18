use crate::models::tags::*;
use std::collections::{HashMap, HashSet};
use crate::core::metadata_source::model::{AlbumDetail, AlbumHit};

#[derive(Debug, Clone, Default)]
pub struct SourceState {
    pub step: u8,
    pub musicbrainz: bool,
    pub query: String,
    /// Fichiers visés (chemins), figés à l'ouverture.
    pub targets: Vec<String>,
    pub hits: Vec<AlbumHit>,
    pub thumbs: HashMap<String, String>,
    pub picked: Option<String>,
    pub searching: bool,
    pub searched: bool,
    pub matching: bool,
    pub error: Option<String>,
    pub seq: u64,
    pub album: Option<AlbumDetail>,
    pub matches: Vec<Match>,
    pub chosen: HashSet<String>,
    pub excluded: HashSet<usize>,
    pub opened: HashSet<String>,
    pub map_open: bool,
    /// 0 compléter, 1 tout, 2 personnalisé.
    pub mode: u8,
    pub cover: Option<RemoteCover>,
    pub cover_loading: bool,
    /// Aperçu : passer à l'étape 2 dès la recherche finie.
    pub auto: bool,
}
