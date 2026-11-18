use serde::Serialize;

#[derive(Serialize, Clone)]
pub struct FileTagsInfo {
    pub path: String,
    pub filename: String,
    pub extension: String,
    pub size: u64,
    pub title: Option<String>,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub album_artist: Option<String>,
    pub year: Option<String>,
    pub genre: Option<String>,
    pub track_number: Option<u16>,
    pub disc_number: Option<u16>,
    pub duration: f32,
    pub bitrate: u32,
    pub sample_rate: u32,
    pub bits_per_sample: u32,
    pub channels: usize,
    pub audio_format: String,
    pub cover: Option<String>,
}
