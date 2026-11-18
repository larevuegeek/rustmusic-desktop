use crate::models::app::SettingsPage;

pub struct NavItem {
    pub page: SettingsPage,
    pub icon: &'static str,
    pub label: String,
    pub hint: String,
    pub active: bool,
    pub group: Option<String>,
}
