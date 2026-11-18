#[derive(Debug, Clone)]
pub struct PlaylistForm {
    pub edit: Option<i64>,
    pub name: String,
    pub desc: String,
    pub color: usize,
    pub icon: usize,
    pub error: Option<String>,
    pub busy: bool,
    pub then_add: Option<(String, String)>,
}
