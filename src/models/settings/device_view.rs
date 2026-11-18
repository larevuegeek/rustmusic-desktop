use crate::models::settings::*;

#[derive(Debug, Clone)]
pub struct DeviceView {
    pub title: String,
    pub icon: &'static str,
    pub is_default: bool,
    pub is_hires: bool,
    pub manufacturer: Option<String>,
    pub raw_name: String,
    pub driver: Option<String>,
    pub channels: String,
    pub banner: Banner,
    pub mix: Option<String>,
    pub formats_title: String,
    pub formats: Vec<(String, Tone)>,
    pub rates_title: String,
    pub rate_groups: Vec<(String, Vec<(String, Tone)>)>,
    pub buffer: Option<String>,
    pub active: bool,
}
