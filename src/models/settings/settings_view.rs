use crate::models::settings::*;

pub struct SettingsView {
    pub nav: Vec<NavItem>,
    pub title: String,
    pub desc: String,
    pub modified_items: Option<(usize, String)>,
    pub groups: Vec<OptionGroup>,
    pub search: String,
    pub none_found: Option<String>,
}
