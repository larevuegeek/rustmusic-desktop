/// Le morceau du lecteur et du hero (`Record`).
#[derive(Debug, Clone, Default)]
pub struct Record {
    pub path: String,
    pub title: String,
    pub artist_name: String,
    pub album: String,
    pub year: String,
    pub duration: f64,
    /// « 03:42 ».
    pub duration_label: String,
    pub cover_path: Option<String>,
}
