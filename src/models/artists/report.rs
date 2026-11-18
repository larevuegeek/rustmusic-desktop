use crate::models::artists::*;

#[derive(Debug, Clone, Default)]
pub struct Report {
    pub checked: Option<i64>,
    pub error: bool,
    pub split: Vec<RuleView>,
    pub whole: usize,
    pub unknown: Vec<RuleView>,
    pub run_unknown: i64,
}
