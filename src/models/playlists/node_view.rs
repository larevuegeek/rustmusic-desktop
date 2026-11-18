use crate::models::playlists::*;

#[derive(Debug, Clone, Default)]
pub struct NodeView {
    pub rule: Option<RuleView>,
    pub group: Option<GroupView>,
}
