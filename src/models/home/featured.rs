/// Le dernier morceau écouté (`featured`).
#[derive(Debug, Clone)]
pub struct Featured {
    pub path: String,
    pub title: String,
    pub artist_name: String,
    pub album: String,
    pub listen: String,
    pub cover_path: Option<String>,
    pub liked: bool,
}
