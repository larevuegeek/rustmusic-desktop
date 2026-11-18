#[derive(Debug, Clone, PartialEq)]
pub enum PlaylistFormMsg {
    /// Nouvelle playlist, puis le morceau (id, chemin) à y verser.
    New(Option<(String, String)>),
    Edit(i64),
    Name(String),
    Desc(String),
    Color(usize),
    Icon(usize),
    Submit,
}
