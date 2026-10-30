use serde::{Deserialize, Serialize};

use crate::entity::audio::audio_tags::AudioTags;

#[derive(Serialize, Deserialize, Debug)]
pub struct AudioFile {
    pub path: String,
    pub audio_format: AudioFormat,
    pub file_size: u64,
    pub duration: f32,
    pub bits_per_sample: u32,
    pub bitrate: u32,
    pub sample_rate: u32,
    pub channels: usize,
    pub track_id: Option<u32>,
    pub tags: AudioTags,
}

impl Default for AudioFile {
    fn default() -> Self {
        Self {
            path: String::new(),
            audio_format: AudioFormat::Unknown,
            file_size: 0,
            duration: 0.0,
            bits_per_sample: 0,
            bitrate: 0,
            sample_rate: 0,
            channels: 0,
            track_id: None,
            tags: AudioTags::new(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub enum AudioFormat {
    MP3,
    FLAC,
    OGG,
    ALAC,
    AAC,
    DFF,
    DSF,
    WAV,
    AIFF,
    Unknown,
}

impl AudioFile {
    pub fn new(path: String) -> Self {
        AudioFile {
            path,
            audio_format: AudioFormat::Unknown,
            file_size: 0,
            duration: 0.0,
            bits_per_sample: 0,
            bitrate: 0,
            sample_rate: 0,
            channels: 0,
            track_id: None,
            tags: AudioTags::new(),
        }
    }
}
