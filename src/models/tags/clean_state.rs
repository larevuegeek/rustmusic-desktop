use crate::models::tags::*;

#[derive(Debug, Clone, Default)]
pub struct CleanState {
    pub rules: Vec<&'static str>,
    pub proposals: Vec<Proposal>,
    pub computing: bool,
    pub error: Option<String>,
    pub seq: u64,
}
