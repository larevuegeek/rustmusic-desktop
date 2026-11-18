use serde::Serialize;

#[derive(Serialize)]
pub struct FormatStat {
    pub name: String,
    pub count: i64,
}

#[derive(Serialize)]
pub struct GenreStat {
    pub name: String,
    pub count: i64,
}

#[derive(Serialize)]
pub struct ArtistStat {
    pub name: String,
    pub count: i64,
}

#[derive(Serialize)]
pub struct TrackPlayStat {
    pub title: String,
    pub artist: String,
    pub play_count: i64,
    pub thumbnail_path: Option<String>,
}

#[derive(Serialize)]
pub struct LibraryStats {
    pub total_tracks: i64,
    pub total_albums: i64,
    pub total_artists: i64,
    pub total_genres: i64,
    pub total_duration_sec: f64,
    pub total_size_bytes: i64,
    pub avg_bitrate: f64,
    pub total_play_count: i64,
    pub formats: Vec<FormatStat>,
    pub top_genres: Vec<GenreStat>,
    pub top_artists: Vec<ArtistStat>,
    pub top_played: Vec<TrackPlayStat>,
    pub quality_hires: i64,
    pub quality_lossless: i64,
    pub quality_lossy: i64,
}
