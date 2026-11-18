use std::collections::HashMap;
use crate::core::radio::directory::RadioResult;
use crate::entity::radio::radio::{RadioRecent, RadioTitle};

#[derive(Default)]
pub struct RadiosPage {
    pub input: String,
    pub results: Option<Vec<RadioResult>>,
    pub search_term: bool,
    pub genre: Option<&'static str>,
    pub popular: Option<Vec<RadioResult>>,
    pub popular_error: bool,
    pub popular_for: (String, Option<&'static str>),
    pub all: bool,
    pub recent_items: Vec<RadioRecent>,
    pub tracks_count: Vec<RadioTitle>,
    pub all_tracks: bool,
    /// Logos de l'annuaire en cache : adresse → chemin local (ou rien).
    pub logos: HashMap<String, Option<String>>,
}
