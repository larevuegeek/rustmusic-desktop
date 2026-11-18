use crate::models::playlists::*;

#[derive(Debug, Clone, Default)]
pub struct GroupView {
    pub head: GroupHead,
    pub nodes: Vec<NodeView>,
}
