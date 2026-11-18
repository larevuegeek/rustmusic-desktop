use crate::models::tags::*;

pub struct Row {
    pub path: String,
    pub file_name: String,
    pub on: bool,
    pub readable: bool,
    pub cover: Option<String>,
    pub cells: Vec<Cell>,
    pub modified: bool,
}
