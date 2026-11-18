use crate::models::podcasts::*;
use karbon_ui::ui::prelude::Color;

pub struct ShowGroup {
    pub id: i64,
    pub title: String,
    pub cover: Option<String>,
    pub color: Color,
    pub subline: String,
    pub confirm: bool,
    pub items: Vec<DownloadRow>,
}
