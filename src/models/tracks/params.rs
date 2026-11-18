/// Paramètres de `get_tracks_paginated` et `get_track_letter_offsets`.
#[derive(Clone)]
pub(crate) struct Params {
    pub(crate) library: i64,
    pub(crate) sort_dir: Option<String>,
    pub(crate) filter: Option<String>,
    pub(crate) missing_cover: Option<bool>,
    pub(crate) quality: Option<String>,
    pub(crate) favorites: Option<bool>,
    pub(crate) genre: Option<String>,
    pub(crate) sort_by: Option<String>,
}
