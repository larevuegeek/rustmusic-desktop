use std::collections::HashMap;

#[derive(Default)]
pub struct YearsPage {
    pub library_id: i64,
    pub recent_items: bool,
    pub choice: HashMap<i64, Option<i64>>,
    pub since: Option<i64>,
    pub start: HashMap<i64, usize>,
}
