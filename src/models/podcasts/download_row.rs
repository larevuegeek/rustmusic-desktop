pub struct DownloadRow {
    pub id: i64,
    pub title: String,
    pub meta: String,
    pub played: bool,
    pub progress: Option<f32>,
    pub size: String,
    pub playback: bool,
    pub playing: bool,
}
