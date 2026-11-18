use crate::models::artists::*;
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, Default)]
pub struct ArtistCheck {
    pub running: bool,
    pub done: i64,
    pub total: i64,
    pub current: String,
    pub live: Vec<LiveItem>,
    pub images: HashMap<String, String>,
    pub started: Option<std::time::Instant>,
    pub run_id: Option<i64>,
    pub error: Option<String>,
    pub one: Option<String>,
    pub report: Option<Report>,
    pub fault: Option<String>,
    pub working: bool,
    pub undone: HashSet<String>,
    pub decided: HashMap<String, String>,
    pub touched: bool,
}
