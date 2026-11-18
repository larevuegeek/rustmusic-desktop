use crate::models::playlists::*;
use serde_json::Value;

#[derive(Debug, Clone, PartialEq)]
pub enum Node {
    Rule { field: String, op: String, value: Value },
    Group(Group),
}
