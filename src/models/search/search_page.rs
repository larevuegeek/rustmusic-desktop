use crate::entity::search::search_result::SearchResult;
use crate::query::track::track_list_row::TrackListRow;

#[derive(Default)]
pub struct SearchPage {
    pub query: String,
    pub results: Vec<SearchResult>,
    pub records: Vec<TrackListRow>,
    pub loading: bool,
    pub filtre: &'static str,
}
