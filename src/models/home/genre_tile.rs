/// Une tuile de genre.
#[derive(Debug, Clone)]
pub struct GenreTile {
    pub name: String,
    pub total: i64,
    pub icon: &'static str,
}
