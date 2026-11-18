use std::collections::HashMap;
use crate::core::podcast::directory::DirectoryResult;
use crate::entity::podcast::podcast::LatestEpisode;

pub struct PodcastsPage {
    pub input: String,
    pub results: Option<Vec<DirectoryResult>>,
    pub search_term: bool,
    pub added: Option<String>,
    pub latest: Vec<LatestEpisode>,
    pub genre: Option<u32>,
    pub popular: Option<Vec<DirectoryResult>>,
    pub popular_error: bool,
    pub popular_for: (String, Option<u32>),
    pub all: bool,
    pub sort: &'static str,
    pub sort_popular: &'static str,
    pub filtre: String,
    pub verification: bool,
    /// Visuels de l'annuaire en cache : adresse → chemin local.
    pub images: HashMap<String, Option<String>>,
}
