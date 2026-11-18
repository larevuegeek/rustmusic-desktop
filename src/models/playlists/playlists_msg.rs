#[derive(Debug, Clone, PartialEq)]
pub enum PlaylistsMsg {
    Kind(&'static str),
    Sort(&'static str),
    ClearFilters,
    Pin(i64, bool),
}
