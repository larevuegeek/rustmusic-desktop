use crate::models::playlists::*;

/// `Group` : `match` vaut « all » (`all = true`) ou « any ».
#[derive(Debug, Clone, PartialEq)]
pub struct Group {
    pub all: bool,
    pub rules: Vec<Node>,
}
