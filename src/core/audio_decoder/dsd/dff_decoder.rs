//! DFF (Philips DSDIFF) container reader: opens a `.dff` file, parses its
//! IFF-style chunks, and streams 1-bit DSD bytes per channel.
//!
//! Two key differences vs DSF that this module handles:
//!
//! 1. **Audio data layout** : DFF stores bytes interleaved per byte
//!    (`[ch0_b0, ch1_b0, ch0_b1, ch1_b1, ...]`) — not per block like DSF.
//!    `read_next_blocks()` deinterleaves into per-channel Vecs.
//!
//! 2. **Bit ordering** : DFF stores bits MSB-first within each byte
//!    (bit 7 = first sample), while DSF is LSB-first. The shared
//!    `DsdToPcmConverter` LUT was built for LSB-first input, so this
//!    reader applies `byte.reverse_bits()` on every byte before exposing
//!    it to the converter.
//!
//! Chunk parsing is delegated to `audio_metadata::file_format::dff`.

use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;

use crate::core::audio_decoder::dsd::dsd_container::DsdContainerReader;
use crate::core::audio_decoder::error::DecodeError;
use crate::core::audio_metadata::file_format::dff::{list_top_chunks, parse_prop};

/// How many DSD bytes per channel we hand the converter per call.
/// Matches the typical DSF block size for consistency.
const READ_CHUNK_BYTES_PER_CHANNEL: usize = 4096;

/// Streaming reader for a DFF file.
pub struct DffDecoder {
    file: File,
    sample_rate: u32,
    channel_count: u8,
    sample_count_per_channel: u64,
    /// Absolute offset of the audio bytes (just after the DSD chunk header).
    data_offset: u64,
    /// Total audio bytes (across all channels).
    data_size: u64,
    /// Bytes already consumed per channel (= bytes_consumed_total / channel_count).
    bytes_consumed_per_channel: u64,
}

impl DffDecoder {
    /// Open a DFF file, parse its container chunks, and position the cursor
    /// at the start of the audio data.
    pub fn open(path: &Path) -> Result<Self, DecodeError> {
        let mut file = File::open(path)?;
        let chunks = list_top_chunks(&mut file)?;

        // PROP/SND : sample rate + channels + compression check
        let prop_loc = chunks
            .iter()
            .find(|c| &c.id == b"PROP")
            .ok_or_else(|| DecodeError::InvalidFormat("DFF: missing PROP chunk".into()))?;
        let props = parse_prop(&mut file, prop_loc)?;

        if &props.compression != b"DSD " {
            return Err(DecodeError::UnsupportedFormat(format!(
                "DFF compression {:?} not supported (only raw DSD)",
                String::from_utf8_lossy(&props.compression)
            )));
        }

        // Audio data chunk
        let dsd_loc = chunks
            .iter()
            .find(|c| &c.id == b"DSD ")
            .ok_or_else(|| DecodeError::InvalidFormat("DFF: missing DSD audio chunk".into()))?;

        let bytes_per_channel = dsd_loc.data_size / props.channel_count as u64;
        let sample_count_per_channel = bytes_per_channel * 8;

        // Position file at the start of the audio bytes
        file.seek(SeekFrom::Start(dsd_loc.data_offset))?;

        Ok(Self {
            file,
            sample_rate: props.sample_rate,
            channel_count: props.channel_count,
            sample_count_per_channel,
            data_offset: dsd_loc.data_offset,
            data_size: dsd_loc.data_size,
            bytes_consumed_per_channel: 0,
        })
    }

    fn bytes_per_channel_total(&self) -> u64 {
        self.data_size / self.channel_count as u64
    }
}

impl DsdContainerReader for DffDecoder {
    fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    fn channel_count(&self) -> u8 {
        self.channel_count
    }

    fn duration_seconds(&self) -> f64 {
        if self.sample_rate == 0 {
            0.0
        } else {
            self.sample_count_per_channel as f64 / self.sample_rate as f64
        }
    }

    fn read_next_blocks(&mut self) -> Result<Option<Vec<Vec<u8>>>, DecodeError> {
        let total_per_channel = self.bytes_per_channel_total();
        let remaining_per_channel =
            total_per_channel.saturating_sub(self.bytes_consumed_per_channel) as usize;
        if remaining_per_channel == 0 {
            return Ok(None);
        }

        let bytes_per_channel = READ_CHUNK_BYTES_PER_CHANNEL.min(remaining_per_channel);
        let total_bytes = bytes_per_channel * self.channel_count as usize;

        let mut buf = vec![0u8; total_bytes];
        self.file.read_exact(&mut buf)?;

        // Deinterleave per byte AND apply bit reversal (DFF is MSB-first;
        // our shared converter LUT expects LSB-first).
        let channel_count = self.channel_count as usize;
        let mut blocks: Vec<Vec<u8>> = (0..channel_count)
            .map(|_| Vec::with_capacity(bytes_per_channel))
            .collect();

        for i in 0..bytes_per_channel {
            let base = i * channel_count;
            for ch in 0..channel_count {
                blocks[ch].push(buf[base + ch].reverse_bits());
            }
        }

        self.bytes_consumed_per_channel += bytes_per_channel as u64;
        Ok(Some(blocks))
    }

    fn seek_to_seconds(&mut self, seconds: f64) -> Result<f64, DecodeError> {
        let seconds = seconds.max(0.0);
        let target_sample = (seconds * self.sample_rate as f64) as u64;
        let target_byte_per_channel = (target_sample / 8).min(self.bytes_per_channel_total());

        let file_offset =
            self.data_offset + target_byte_per_channel * self.channel_count as u64;
        self.file.seek(SeekFrom::Start(file_offset))?;
        self.bytes_consumed_per_channel = target_byte_per_channel;

        let actual_sample = target_byte_per_channel * 8;
        Ok(actual_sample as f64 / self.sample_rate as f64)
    }
}
