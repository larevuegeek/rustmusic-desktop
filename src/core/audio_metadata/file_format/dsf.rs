//! DSF (Sony Direct Stream Digital) file format parser.
//!
//! Reference: Sony, "DSF File Format Specification", 2005.
//!
//! Layout of a `.dsf` file (all integers little-endian):
//!
//! ```text
//! +------------------------------------------------------+ offset 0
//! | DSD chunk (28 bytes)                                 |
//! |   "DSD " (4) | chunk size = 28 (8) | total file size |
//! |   (8) | metadata chunk pointer (8)                   |
//! +------------------------------------------------------+ offset 28
//! | fmt chunk (52 bytes)                                 |
//! |   "fmt " (4) | chunk size = 52 (8) | format ver (4)  |
//! |   format id (4) | channel type (4) | channel num (4) |
//! |   sample rate (4) | bits/sample (4) | sample count   |
//! |   per channel (8) | block size per channel (4) | rsv |
//! +------------------------------------------------------+ offset 80
//! | data chunk                                           |
//! |   "data" (4) | chunk size (8) | DSD samples (1-bit)  |
//! +------------------------------------------------------+
//! | metadata chunk (optional, raw ID3v2 blob)            |
//! +------------------------------------------------------+
//! ```

use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;

use crate::core::audio_metadata::extractor::error::ExtractError;
use crate::core::audio_metadata::tag_format::id3v2;
use crate::entity::audio::audio_file::{AudioFile, AudioFormat};
use crate::entity::audio::audio_tags::AudioTags;

/// Parsed fields we keep from the DSD chunk.
pub struct DsdChunk {
    pub total_file_size: u64,
    /// Absolute offset of the metadata (ID3v2) chunk, or 0 if no metadata.
    pub metadata_offset: u64,
}

/// Parsed fields we keep from the fmt chunk.
pub struct FmtChunk {
    pub channel_type: u32,
    pub channel_count: u32,
    /// In Hz, e.g. 2_822_400 for DSD64, 5_644_800 for DSD128.
    pub sample_rate: u32,
    /// Always 1 for raw DSD.
    pub bits_per_sample: u32,
    /// Total number of 1-bit samples per channel.
    pub sample_count: u64,
    /// Size in bytes of one block per channel (typically 4096).
    pub block_size_per_channel: u32,
}

/// Public entry point: read a DSF file and return a fully-populated `AudioFile`.
pub fn extract(path: &Path) -> Result<AudioFile, ExtractError> {
    let mut file = File::open(path)?;

    // 1. Parse the two header chunks at known offsets.
    let dsd = parse_dsd_chunk(&mut file)?;
    let fmt = parse_fmt_chunk(&mut file)?;

    // 2. Derived values: duration (s) and bitrate (kbps).
    let duration = if fmt.sample_rate > 0 {
        fmt.sample_count as f64 / fmt.sample_rate as f64
    } else {
        0.0
    };
    let bitrate = ((fmt.sample_rate as u64
        * fmt.channel_count as u64
        * fmt.bits_per_sample as u64)
        / 1000) as u32;

    // 3. ID3v2 metadata if the DSD chunk pointed at one.
    let tags = if dsd.metadata_offset > 0 {
        read_id3v2_at_offset(&mut file, dsd.metadata_offset)?
    } else {
        AudioTags::new()
    };

    // 4. File size from the OS (the DSD chunk also stores it but trust the FS).
    let file_size = std::fs::metadata(path)?.len();

    Ok(AudioFile {
        path: path.to_string_lossy().into_owned(),
        audio_format: AudioFormat::DSF,
        file_size,
        duration: duration as f32,
        bits_per_sample: fmt.bits_per_sample,
        bitrate,
        sample_rate: fmt.sample_rate,
        channels: fmt.channel_count as usize,
        track_id: None,
        tags,
    })
}

// ─── Chunk parsers ───────────────────────────────────────────────────

pub fn parse_dsd_chunk(file: &mut File) -> Result<DsdChunk, ExtractError> {
    let mut buf = [0u8; 28];
    file.read_exact(&mut buf)?;

    if &buf[0..4] != b"DSD " {
        return Err(ExtractError::InvalidHeader(format!(
            "Expected DSD magic 'DSD ', got {:?}",
            &buf[0..4]
        )));
    }

    let chunk_size = read_u64_le(&buf[4..12]);
    if chunk_size != 28 {
        return Err(ExtractError::InvalidHeader(format!(
            "Expected DSD chunk size 28, got {chunk_size}"
        )));
    }

    let total_file_size = read_u64_le(&buf[12..20]);
    let metadata_offset = read_u64_le(&buf[20..28]);

    Ok(DsdChunk {
        total_file_size,
        metadata_offset,
    })
}

pub fn parse_fmt_chunk(file: &mut File) -> Result<FmtChunk, ExtractError> {
    let mut buf = [0u8; 52];
    file.read_exact(&mut buf)?;

    if &buf[0..4] != b"fmt " {
        return Err(ExtractError::InvalidHeader(format!(
            "Expected fmt magic 'fmt ', got {:?}",
            &buf[0..4]
        )));
    }

    let chunk_size = read_u64_le(&buf[4..12]);
    if chunk_size != 52 {
        return Err(ExtractError::InvalidHeader(format!(
            "Expected fmt chunk size 52, got {chunk_size}"
        )));
    }

    // format_version at 12..16, ignored
    let format_id = read_u32_le(&buf[16..20]);
    if format_id != 0 {
        return Err(ExtractError::InvalidHeader(format!(
            "Unsupported DSF format ID {format_id} (only raw DSD = 0 is handled)"
        )));
    }

    Ok(FmtChunk {
        channel_type: read_u32_le(&buf[20..24]),
        channel_count: read_u32_le(&buf[24..28]),
        sample_rate: read_u32_le(&buf[28..32]),
        bits_per_sample: read_u32_le(&buf[32..36]),
        sample_count: read_u64_le(&buf[36..44]),
        block_size_per_channel: read_u32_le(&buf[44..48]),
    })
}

/// Seek to `offset` and try to read an ID3v2 tag block.
/// Returns empty tags if no valid ID3 magic is present.
fn read_id3v2_at_offset(file: &mut File, offset: u64) -> Result<AudioTags, ExtractError> {
    file.seek(SeekFrom::Start(offset))?;

    let mut blob = Vec::new();
    file.read_to_end(&mut blob)?;

    if blob.len() < 10 || &blob[0..3] != b"ID3" {
        return Ok(AudioTags::new());
    }

    let parsed = id3v2::parse(&blob)?;
    Ok(id3v2::to_audio_tags(&parsed))
}

// ─── Endianness helpers (no `byteorder` dependency) ──────────────────

fn read_u32_le(bytes: &[u8]) -> u32 {
    u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]])
}

fn read_u64_le(bytes: &[u8]) -> u64 {
    u64::from_le_bytes([
        bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
    ])
}
