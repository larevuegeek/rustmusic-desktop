use crate::models::radios::*;
use crate::models::app::Choice;

pub struct RadiosView {
    pub active: bool,
    pub count: Option<(String, String)>,
    pub input: String,
    pub address: bool,
    pub search_term: bool,
    pub results: Option<Vec<RadioItem>>,
    pub favorites: Vec<RadioItem>,
    pub loaded: bool,
    pub recent_items: Vec<RadioItem>,
    pub tracks_count: Vec<TitleItem>,
    pub tracks_more: Option<String>,
    pub all_tracks: bool,
    pub country_hint: String,
    pub country_label: String,
    pub country: Vec<Choice>,
    pub genres: Vec<(Option<&'static str>, String, bool)>,
    pub popular: Option<Vec<RadioItem>>,
    pub popular_error: bool,
    pub more: Option<String>,
}
